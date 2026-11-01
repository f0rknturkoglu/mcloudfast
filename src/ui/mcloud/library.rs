//! mcloud's library views (user.css `#am-library`): Recently Added,
//! Artists, Albums and All Playlists share one page. A title, a filter and
//! the order, how many items there are, then a grid of covers at least
//! 150pt wide, 22pt apart and 28pt between rows, each named under it.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Vec2, pos2, vec2};

use crate::app::App;
use crate::i18n::{gettext, ngettext};
use crate::model::{Action, Page};
use crate::theme::{self, Icon};
use crate::ui::widgets;

const MIN_CARD: f32 = 150.0;
const COLUMN_GAP: f32 = 22.0;
const ROW_GAP: f32 = 28.0;
const NAME: f32 = 14.0;
const SUB: f32 = 12.0;

/// One cover in the grid.
pub(in crate::ui) struct Entry {
    pub uri: String,
    pub name: String,
    pub subtitle: String,
    pub image: Option<String>,
    pub page: Page,
    /// Artists are round.
    pub round: bool,
}

#[derive(Clone, Default)]
struct Tools {
    filter: String,
    by_name: bool,
}

/// The page above the footer: the title, the tools and the grid.
pub(in crate::ui) fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    page: &Page,
    title: &str,
    entries: &[Entry],
) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(11.0);
    let galley = ui
        .painter()
        .layout_job(egui::text::LayoutJob::single_section(
            title.to_string(),
            egui::TextFormat {
                font_id: theme::semibold(28.0),
                color: palette.text,
                extra_letter_spacing: -0.04 * 28.0,
                ..Default::default()
            },
        ));
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, palette.text);

    // The tools are kept for the session, one set per view.
    let tools_id = Id::new(("mcloud-library-tools", page.encode()));
    let mut tools = ui.data(|data| data.get_temp::<Tools>(tools_id).unwrap_or_default());
    if !entries.is_empty() {
        // mcloud's 8pt under the title and 14pt over the tools, less
        // the title's line and the row spacing.
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let field = egui::TextEdit::singleline(&mut tools.filter)
                .id(tools_id.with("filter"))
                .hint_text(gettext(locale, "Filter").as_ref())
                .font(theme::regular(13.0))
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::symmetric(10, 0))
                .vertical_align(egui::Align::Center)
                .desired_width(260.0 - 20.0);
            let (frame, _) = ui.allocate_exact_size(vec2(260.0, 30.0), Sense::hover());
            ui.painter().rect(
                frame,
                CornerRadius::same(8),
                Color32::from_white_alpha(10),
                egui::Stroke::new(1.0, Color32::from_white_alpha(20)),
                egui::StrokeKind::Inside,
            );
            ui.put(frame, field);

            let labels = [gettext(locale, "Library order"), gettext(locale, "Name")];
            let chosen = labels[usize::from(tools.by_name)].clone();
            let galley = crate::bidi::layout_line(
                ui.painter(),
                chosen.as_ref(),
                theme::regular(13.0),
                palette.text,
            );
            let (sort, response) =
                ui.allocate_exact_size(vec2(galley.size().x + 36.0, 30.0), Sense::click());
            ui.painter().rect(
                sort,
                CornerRadius::same(8),
                Color32::from_rgb(0x2b, 0x2b, 0x2b),
                egui::Stroke::new(1.0, Color32::from_white_alpha(20)),
                egui::StrokeKind::Inside,
            );
            ui.painter().galley(
                pos2(sort.left() + 8.0, sort.center().y - galley.size().y / 2.0),
                galley,
                palette.text,
            );
            theme::paint_icon(
                ui,
                Icon::ChevronDown,
                Rect::from_center_size(
                    pos2(sort.right() - 14.0, sort.center().y),
                    Vec2::splat(12.0),
                ),
                12.0,
                palette.secondary,
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, chosen.as_ref())
            });
            theme::focus_ring(ui, &response);
            egui::Popup::menu(&response)
                .frame(widgets::menu_frame(&palette))
                .show(|ui| {
                    for (index, label) in labels.iter().enumerate() {
                        let picked = usize::from(tools.by_name) == index;
                        let check = picked.then_some(Icon::Check);
                        if widgets::menu_item(ui, &palette, check, label) {
                            tools.by_name = index == 1;
                            ui.close();
                        }
                    }
                });
        });
        ui.add_space(4.0);
    } else {
        ui.add_space(8.0);
    }

    let shown = visible(entries, &tools);
    let status = items(locale, entries.len());
    let status = if tools.filter.trim().is_empty() {
        status
    } else if shown.is_empty() {
        gettext(locale, "No matches.").into_owned()
    } else {
        format!("{} / {status}", shown.len())
    };
    if !entries.is_empty() {
        theme::text(ui, status, theme::regular(13.0), palette.secondary);
        ui.add_space(16.0);
    }
    ui.data_mut(|data| data.insert_temp(tools_id, tools));
    grid(app, ui, entries, &shown);
}

/// `42 items`.
fn items(locale: crate::i18n::Locale, count: usize) -> String {
    // Translators: How many albums, artists or playlists a library view
    // holds. Keep {count}.
    ngettext(locale, "{count} item", "{count} items", count as u32)
        .replace("{count}", &crate::util::format_count(count as u64))
}

/// The entries the filter lets through, in the order chosen.
fn visible(entries: &[Entry], tools: &Tools) -> Vec<usize> {
    let query = tools.filter.trim().to_lowercase();
    let mut shown: Vec<usize> = (0..entries.len())
        .filter(|&index| query.is_empty() || entries[index].name.to_lowercase().contains(&query))
        .collect();
    if tools.by_name {
        shown.sort_by_cached_key(|&index| entries[index].name.to_lowercase());
    }
    shown
}

/// How many columns fit, and how wide each is.
fn columns(width: f32) -> (usize, f32) {
    let count = (((width + COLUMN_GAP) / (MIN_CARD + COLUMN_GAP)).floor() as usize).max(1);
    let card = (width - COLUMN_GAP * (count as f32 - 1.0)) / count as f32;
    (count, card)
}

fn grid(app: &mut App, ui: &mut egui::Ui, entries: &[Entry], shown: &[usize]) {
    let width = ui.available_width();
    let (count, card) = columns(width);
    // The cover, 4pt, the name, 5pt and the subtitle.
    let height = card + 4.0 + 5.0 + NAME * 1.25 + 5.0 + SUB * 1.25;
    for row in shown.chunks(count) {
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        if ui.is_rect_visible(rect) {
            for (column, &index) in row.iter().enumerate() {
                let x = rect.left() + column as f32 * (card + COLUMN_GAP);
                let cell = Rect::from_min_size(pos2(x, rect.top()), vec2(card, height));
                cover_card(app, ui, &entries[index], cell, card);
            }
        }
        ui.add_space(ROW_GAP - ui.spacing().item_spacing.y);
    }
}

fn cover_card(app: &mut App, ui: &mut egui::Ui, entry: &Entry, cell: Rect, side: f32) {
    let palette = app.palette;
    let response = ui.interact(
        cell,
        Id::new(("mcloud-library-card", &entry.uri)),
        Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &entry.name));
    let art = Rect::from_min_size(cell.min, Vec2::splat(side));
    let radius = if entry.round { side / 2.0 } else { 8.0 };
    ui.painter().rect_filled(
        art,
        CornerRadius::same(radius.min(255.0) as u8),
        Color32::from_rgb(0x30, 0x30, 0x30),
    );
    let placeholder = if entry.round {
        Icon::McloudArtist
    } else {
        Icon::McloudAlbum
    };
    widgets::paint_cover(
        ui,
        &palette,
        entry.image.as_deref(),
        art,
        radius,
        placeholder,
        Some(app.backend.art()),
    );
    if response.hovered() {
        ui.painter().rect_filled(
            art,
            CornerRadius::same(radius.min(255.0) as u8),
            Color32::from_white_alpha(14),
        );
    }
    let text_width = cell.width();
    let name = widgets::ellipsized(
        ui,
        &entry.name,
        theme::medium(NAME),
        palette.text,
        text_width,
        1,
    );
    let name_top = art.bottom() + 4.0 + 5.0;
    ui.painter()
        .galley(pos2(cell.left(), name_top), name, palette.text);
    let subtitle = widgets::ellipsized(
        ui,
        &entry.subtitle,
        theme::regular(SUB),
        palette.secondary,
        text_width,
        1,
    );
    ui.painter().galley(
        pos2(cell.left(), name_top + NAME * 1.25 + 5.0),
        subtitle,
        palette.secondary,
    );
    theme::focus_ring(ui, &response);
    if response.clicked() {
        app.actions.push(Action::Open(entry.page.clone()));
    }
    egui::Popup::context_menu(&response)
        .id(ui.make_persistent_id(("mcloud-library-menu", &entry.uri)))
        .frame(widgets::menu_frame(&palette))
        .show(|ui| widgets::context_menu_items(ui, app, &entry.uri, &entry.name, None));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> Entry {
        Entry {
            uri: format!("spotify:album:{name}"),
            name: name.into(),
            subtitle: String::new(),
            image: None,
            page: Page::Album(name.into()),
            round: false,
        }
    }

    /// Covers are at least 150pt wide and share the row's spare width.
    #[test]
    fn the_grid_fits_as_many_150pt_covers_as_it_can() {
        assert_eq!(columns(100.0), (1, 100.0));
        let (count, card) = columns(1000.0);
        assert_eq!(count, 5);
        assert!((card - (1000.0 - 4.0 * 22.0) / 5.0).abs() < 0.01);
    }

    /// The filter matches names in any case; by name sorts them, library
    /// order keeps Spotify's.
    #[test]
    fn the_filter_and_order_pick_the_covers_shown() {
        let entries = [entry("Zebra"), entry("apple"), entry("Banana")];
        let mut tools = Tools::default();
        assert_eq!(visible(&entries, &tools), [0, 1, 2]);
        tools.by_name = true;
        assert_eq!(visible(&entries, &tools), [1, 2, 0]);
        tools.filter = "AN".into();
        assert_eq!(visible(&entries, &tools), [2]);
    }
}

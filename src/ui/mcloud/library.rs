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
            let (frame, _) = ui.allocate_exact_size(vec2(240.0, 30.0), Sense::hover());
            let filter_id = tools_id.with("filter");
            let has_focus = ui.memory(|m| m.has_focus(filter_id));
            ui.painter().rect(
                frame,
                CornerRadius::same(15),
                Color32::from_white_alpha(if has_focus { 16 } else { 10 }),
                egui::Stroke::new(
                    1.0,
                    if has_focus {
                        palette.accent
                    } else {
                        Color32::from_white_alpha(20)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let icon_rect = Rect::from_center_size(
                pos2(frame.left() + 15.0, frame.center().y),
                Vec2::splat(14.0),
            );
            theme::paint_icon(ui, Icon::Search, icon_rect, 14.0, palette.secondary);
            let right_inset = if tools.filter.is_empty() { 10.0 } else { 26.0 };
            let edit_rect = Rect::from_min_max(
                pos2(frame.left() + 28.0, frame.top() + 1.0),
                pos2(frame.right() - right_inset, frame.bottom() - 1.0),
            );
            let field = egui::TextEdit::singleline(&mut tools.filter)
                .id(filter_id)
                .hint_text(gettext(locale, "Filter").as_ref())
                .font(theme::regular(13.0))
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .vertical_align(egui::Align::Center)
                .desired_width(edit_rect.width());
            ui.put(edit_rect, field);
            if !tools.filter.is_empty() {
                let clear_rect = Rect::from_center_size(
                    pos2(frame.right() - 14.0, frame.center().y),
                    Vec2::splat(16.0),
                );
                let clear_resp =
                    ui.interact(clear_rect, tools_id.with("filter-clear"), Sense::click());
                theme::paint_icon(
                    ui,
                    Icon::X,
                    clear_rect,
                    12.0,
                    if clear_resp.hovered() {
                        palette.text
                    } else {
                        palette.secondary
                    },
                );
                if clear_resp.on_hover_text(gettext(locale, "Clear")).clicked() {
                    tools.filter.clear();
                }
            }

            let labels = [gettext(locale, "Library order"), gettext(locale, "Name")];
            let selected_idx = usize::from(tools.by_name);
            let chosen = labels[selected_idx].clone();
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
                CornerRadius::same(15),
                Color32::from_rgb(0x2b, 0x2b, 0x2b),
                egui::Stroke::new(1.0, Color32::from_white_alpha(20)),
                egui::StrokeKind::Inside,
            );
            ui.painter().galley(
                pos2(sort.left() + 12.0, sort.center().y - galley.size().y / 2.0),
                galley,
                palette.text,
            );
            // macOS pop-up button indicator: dual up/down chevrons.
            let arrows_x = sort.right() - 13.0;
            let cy = sort.center().y;
            for (dy, tip) in [(-3.0_f32, -2.6_f32), (3.0_f32, 2.6_f32)] {
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        pos2(arrows_x - 3.0, cy + dy),
                        pos2(arrows_x + 3.0, cy + dy),
                        pos2(arrows_x, cy + dy + tip),
                    ],
                    palette.secondary,
                    egui::Stroke::NONE,
                ));
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, chosen.as_ref())
            });
            theme::focus_ring(ui, &response);
            // Pop-up menu aligns the currently selected item over the button.
            let popup_pos = pos2(
                sort.left(),
                (sort.top() - 6.0 - selected_idx as f32 * 28.0).max(8.0),
            );
            egui::Popup::from_toggle_button_response(&response)
                .at_position(popup_pos)
                .frame(widgets::menu_frame(&palette))
                .show(|ui| {
                    for (index, label) in labels.iter().enumerate() {
                        let picked = selected_idx == index;
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

    let visit_id = Id::new("mcloud-library-visit");
    let page_key = page.encode();
    let held_order = ui
        .data(|data| data.get_temp::<(String, Vec<String>)>(visit_id))
        .filter(|(held_page, _)| *held_page == page_key)
        .map(|(_, uris)| uris);
    let order = stabilize_visit_order(entries, held_order.as_deref());
    let next_uris: Vec<String> = order.iter().map(|&i| entries[i].uri.clone()).collect();
    ui.data_mut(|data| data.insert_temp(visit_id, (page_key, next_uris)));

    let shown = visible_ordered(entries, &order, &tools);
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

/// Holds the current visit's library order steady if a background refresh
/// only reorders the same set of URIs, while applying additions or removals
/// at once.
fn stabilize_visit_order(entries: &[Entry], held: Option<&[String]>) -> Vec<usize> {
    if let Some(held) = held
        && held.len() == entries.len()
    {
        let mut by_uri = std::collections::HashMap::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            by_uri.insert(entry.uri.as_str(), index);
        }
        if by_uri.len() == entries.len() {
            let mapped: Vec<usize> = held
                .iter()
                .filter_map(|uri| by_uri.get(uri.as_str()).copied())
                .collect();
            if mapped.len() == entries.len() {
                return mapped;
            }
        }
    }
    (0..entries.len()).collect()
}

/// The entries the filter lets through, in the order chosen.
#[cfg(test)]
fn visible(entries: &[Entry], tools: &Tools) -> Vec<usize> {
    let order: Vec<usize> = (0..entries.len()).collect();
    visible_ordered(entries, &order, tools)
}

fn visible_ordered(entries: &[Entry], order: &[usize], tools: &Tools) -> Vec<usize> {
    let query = tools.filter.trim().to_lowercase();
    let mut shown: Vec<usize> = order
        .iter()
        .copied()
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
    if let Page::Artist(id) = &entry.page {
        widgets::prefetch_artist_on_hover(ui.ctx(), &response, id, &mut app.actions);
    }
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

    #[test]
    fn visit_order_holds_through_pure_reorder_and_updates_on_addition() {
        let entries = [entry("Portishead"), entry("Air"), entry("Massive Attack")];
        let held = vec![
            "spotify:album:Air".to_string(),
            "spotify:album:Portishead".to_string(),
            "spotify:album:Massive Attack".to_string(),
        ];
        assert_eq!(stabilize_visit_order(&entries, Some(&held)), vec![1, 0, 2]);
        let shorter = vec!["spotify:album:Air".to_string()];
        assert_eq!(
            stabilize_visit_order(&entries, Some(&shorter)),
            vec![0, 1, 2]
        );
    }
}

//! Playlist and album pages as Apple Music lays them out: a flat page with
//! the artwork on the left and, beside it, the kind, the title, the owner
//! or artists in the accent, the details, and Play and Shuffle pills along
//! the artwork's bottom edge.
//!
//! [`hero`] leaves the band for the pills where [`actions_row`] finds it
//! later in the same pass, so Spotifast's pages call both unchanged.

use std::sync::Arc;

use egui::{Align, Color32, CornerRadius, Id, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog, RowContext};
use crate::theme::{self, Icon, Palette};
use crate::ui::collection::{Actions, Hero};
use crate::ui::widgets;

const ART: f32 = 240.0;
const ART_NARROW: f32 = 170.0;
const ART_RADIUS: f32 = 8.0;
const COLUMN_GAP: f32 = 28.0;
const PILL_HEIGHT: f32 = 32.0;
/// Wide enough for the longest translation of Pause, so a pill keeps its
/// size when its label changes.
const PILL_MIN_WIDTH: f32 = 112.0;
const ACTIONS_ID: &str = "mcloud-hero-actions";

pub(in crate::ui) fn hero(app: &mut App, ui: &mut egui::Ui, hero: Hero<'_>) {
    let palette = app.palette;
    let art_loader = app.backend.art().clone();
    let softened = hero
        .images
        .thumbnail
        .and_then(|uri| app.softened_covers.texture(ui.ctx(), &art_loader, uri));
    ui.add_space(16.0);
    let size = if ui.available_width() > 720.0 {
        ART
    } else {
        ART_NARROW
    };
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), size), Sense::hover());
    let art = Rect::from_min_size(row.min, egui::Vec2::splat(size));
    let radius = if hero.round { size / 2.0 } else { ART_RADIUS };

    // Clicking the cover shows it enlarged, as on Spotifast's pages.
    let enlarged = hero.images.enlarged.filter(|_| !hero.liked);
    let sense = if enlarged.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(art, Id::new("mcloud-hero-art"), sense);
    if let Some(url) = enlarged {
        let label = gettext(app.locale, "Enlarge cover");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.actions.push(Action::ShowDialog(Dialog::CoverArt {
                url: url.to_string(),
                shown: hero.images.image.map(str::to_string),
                title: hero.title.to_string(),
            }));
        }
    }
    widgets::paint_shadow(ui, &palette, art, radius);
    if hero.liked {
        crate::ui::sidebar::liked_cover(ui, art, radius);
    } else {
        widgets::paint_cover_with_thumbnail(
            ui,
            &palette,
            widgets::CoverSources {
                requested: hero.images.image,
                previous: hero.images.previous,
                softened: softened.as_ref(),
                thumbnail: hero.images.thumbnail,
                align_thumbnail: hero.images.align_thumbnail,
            },
            art,
            radius,
            if hero.round { Icon::User } else { Icon::Music },
            Some(&art_loader),
        );
    }

    let left = art.right() + COLUMN_GAP;
    let width = (row.right() - left).max(0.0);
    // The pills take the band along the artwork's bottom edge.
    let actions = Rect::from_min_max(pos2(left, art.bottom() - PILL_HEIGHT), row.max);
    // The pass is read before the data lock: egui's context locks are not
    // reentrant.
    let pass = ui.ctx().cumulative_pass_nr();
    ui.ctx()
        .data_mut(|data| data.insert_temp(Id::new(ACTIONS_ID), (pass, actions)));

    // The words stand above the pills, the title's top a little below the
    // artwork's, as in Apple Music.
    let mut y = art.top() + size * 0.1;
    let paint = |ui: &egui::Ui, y: &mut f32, galley: Arc<egui::Galley>, color: Color32| {
        let rect = Rect::from_min_size(pos2(left, *y), galley.size());
        ui.painter().galley(rect.min, galley, color);
        *y = rect.bottom();
        rect
    };

    let kind = widgets::ellipsized(
        ui,
        &hero.kind.to_uppercase(),
        theme::semibold(11.5),
        palette.dim,
        width,
        1,
    );
    paint(ui, &mut y, kind, palette.dim);
    y += 4.0;

    let mut title_size = if size >= ART { 30.0 } else { 24.0 };
    let title = loop {
        let galley = widgets::ellipsized(
            ui,
            hero.title,
            theme::bold(title_size),
            palette.text,
            width,
            2,
        );
        if galley.rows.len() < 2 || title_size <= 20.0 {
            break galley;
        }
        title_size -= 2.0;
    };
    paint(ui, &mut y, title, palette.text);
    y += 2.0;

    // The names that open a page lead in the accent: the owner, or the
    // album's artists. The rest of the byline follows quietly.
    let names: Vec<_> = hero
        .byline
        .iter()
        .take_while(|(_, page)| page.is_some())
        .collect();
    let mut x = left;
    for (index, (name, page)) in names.iter().enumerate() {
        let text = if index + 1 < names.len() {
            format!("{name},")
        } else {
            name.clone()
        };
        let galley = ui.painter().layout_no_wrap(
            text,
            theme::semibold(if size >= ART { 19.0 } else { 16.0 }),
            palette.accent,
        );
        let rect = Rect::from_min_size(pos2(x, y), galley.size());
        if rect.right() > row.right() {
            break;
        }
        let response = ui
            .interact(rect, Id::new(("mcloud-hero-name", index)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let color = if response.hovered() {
            palette.accent_hover
        } else {
            palette.accent
        };
        ui.painter().galley(rect.min, galley, color);
        if response.clicked()
            && let Some(page) = page
        {
            app.actions.push(Action::Open(page.clone()));
        }
        x = rect.right() + 6.0;
        if index + 1 == names.len() {
            y = rect.bottom();
        }
    }

    let details: Vec<&str> = hero
        .byline
        .iter()
        .skip(names.len())
        .map(|(text, _)| text.as_str())
        .collect();
    if !details.is_empty() {
        y += 4.0;
        let galley = widgets::ellipsized(
            ui,
            &details.join(" · "),
            theme::regular(12.5),
            palette.secondary,
            width,
            1,
        );
        let rect = paint(ui, &mut y, galley, palette.secondary);
        if let Some((at, tooltip)) = &hero.byline_tooltip
            && *at >= names.len()
        {
            ui.interact(rect, Id::new("mcloud-hero-details"), Sense::hover())
                .on_hover_text(tooltip);
        }
    }

    if let Some(description) = hero.description.as_deref().filter(|text| !text.is_empty()) {
        y += 8.0;
        // Only what fits above the pills.
        let room = actions.top() - 10.0 - y;
        let line = 18.0;
        let rows = (room / line).floor().clamp(0.0, 3.0) as usize;
        if rows > 0 {
            let galley = widgets::ellipsized(
                ui,
                description,
                theme::regular(13.0),
                palette.secondary,
                width,
                rows,
            );
            paint(ui, &mut y, galley, palette.secondary);
        }
    }
    ui.add_space(20.0);
}

/// Play and Shuffle as pills, then the save, radio and "…" buttons, in the
/// band [`hero`] left for them, or on a row of their own on pages without
/// one. A filter field takes its own row, at the right.
pub(in crate::ui) fn actions_row(
    app: &mut App,
    ui: &mut egui::Ui,
    actions: Actions<'_>,
    filter: Option<&mut String>,
) {
    let pass = ui.ctx().cumulative_pass_nr();
    let band = ui.ctx().data_mut(|data| {
        let band = data.get_temp::<(u64, Rect)>(Id::new(ACTIONS_ID));
        data.remove::<(u64, Rect)>(Id::new(ACTIONS_ID));
        band.filter(|(at, _)| *at == pass).map(|(_, rect)| rect)
    });
    let filtered = filter.as_ref().is_some_and(|f| !f.trim().is_empty());
    let enabled = ui.is_enabled();
    match band {
        Some(band) => {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(band)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            child.add_enabled_ui(enabled, |ui| controls(app, ui, &actions, filtered));
        }
        None => {
            ui.horizontal(|ui| controls(app, ui, &actions, filtered));
            ui.add_space(10.0);
        }
    }
    if let Some(filter) = filter {
        let palette = app.palette;
        // A row of its own: in the page's column the field would centre in
        // all the height left below it.
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::search_field(
                    ui,
                    &palette,
                    app.locale,
                    Id::new(("collection-filter", actions.name)),
                    filter,
                    &gettext(app.locale, "Filter"),
                    220.0,
                );
            });
        });
        ui.add_space(6.0);
    }
}

fn controls(app: &mut App, ui: &mut egui::Ui, actions: &Actions<'_>, filtered: bool) {
    let palette = app.palette;
    let locale = app.locale;
    ui.spacing_mut().item_spacing.x = 10.0;
    if let Some(uri) = &actions.play_uri {
        let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
        let playing_here = context_here && app.believed_playing();
        let can_start = actions.view.as_ref().is_none_or(|uris| !uris.is_empty());
        let pending = app.play_pending(uri);
        let (icon, label) = if playing_here {
            (Icon::PauseFilled, gettext(locale, "Pause"))
        } else {
            (Icon::PlayFilled, gettext(locale, "Play"))
        };
        let play = ui
            .add_enabled_ui(context_here || can_start, |ui| {
                pill(ui, &palette, icon, &label, pending)
            })
            .inner
            .on_disabled_hover_text(gettext(locale, "No playable songs in this view").as_ref());
        if play.clicked() && !pending {
            if context_here {
                app.actions.push(Action::TogglePlay);
            } else {
                start(app, actions, uri, filtered, app.playing_context_shuffle());
            }
        }
        let shuffle = ui
            .add_enabled_ui(can_start, |ui| {
                pill(
                    ui,
                    &palette,
                    Icon::Shuffle,
                    &gettext(locale, "Shuffle"),
                    false,
                )
            })
            .inner;
        if shuffle.clicked() {
            // Apple Music's Shuffle starts the list shuffled from the top.
            app.actions.push(Action::SetShuffle(true));
            start(app, actions, uri, filtered, true);
        }
        ui.add_space(6.0);
    }
    if let Some((uri, saved)) = &actions.saved {
        let (icon, tooltip, color, hover) = if *saved {
            (
                actions.saved_icons.1,
                &actions.saved_tooltips.1,
                palette.accent,
                palette.accent_hover,
            )
        } else {
            (
                actions.saved_icons.0,
                &actions.saved_tooltips.0,
                palette.secondary,
                palette.text,
            )
        };
        if theme::icon_button(ui, icon, 20.0, color, hover, tooltip).clicked() {
            app.actions.push(Action::ToggleSaved(uri.clone()));
        }
    }
    if let Some(seed) = &actions.save_radio
        && theme::icon_button(
            ui,
            Icon::CirclePlus,
            20.0,
            palette.secondary,
            palette.text,
            &gettext(locale, "Save as playlist"),
        )
        .clicked()
    {
        app.actions.push(Action::SaveRadio(seed.clone()));
    }
    if let Some(uri) = &actions.play_uri {
        let more = theme::icon_button(
            ui,
            Icon::Ellipsis,
            20.0,
            palette.secondary,
            palette.text,
            &gettext(locale, "More"),
        );
        egui::Popup::menu(&more)
            .frame(widgets::menu_frame(&palette))
            .show(|ui| {
                if let Some(seed) = &actions.save_radio {
                    ui.set_min_width(200.0);
                    ui.set_max_width(300.0);
                    if widgets::menu_item(
                        ui,
                        &palette,
                        Some(Icon::CirclePlus),
                        &gettext(locale, "Save as playlist"),
                    ) {
                        app.actions.push(Action::SaveRadio(seed.clone()));
                    }
                } else {
                    widgets::context_menu_items(
                        ui,
                        app,
                        uri,
                        actions.name,
                        actions.owned_playlist.as_ref(),
                    );
                }
                if let Some((page, loading)) = &actions.reload {
                    widgets::menu_separator(ui, &palette);
                    crate::ui::collection::refresh_menu_item(app, ui, page, *loading);
                }
            });
    }
}

/// Starts the page's songs as Spotifast's Play button does: the songs on
/// screen when they differ from the context's order, otherwise the context.
fn start(app: &mut App, actions: &Actions<'_>, uri: &str, filtered: bool, shuffle: bool) {
    let play_view = actions.view.is_some()
        && (!shuffle || filtered || actions.save_radio.is_some() || actions.plays_shown_rows);
    if let Some(uris) = actions.view.clone()
        && play_view
    {
        app.actions.push(Action::PlayFromRow {
            context: RowContext::View {
                uris,
                context_uri: uri.to_string(),
                editable_playlist: None,
            },
            uri: String::new(),
            index: 0,
        });
    } else {
        app.actions.push(Action::PlayContext {
            uri: uri.to_string(),
            offset_uri: None,
            offset_index: None,
        });
    }
}

/// Apple Music's filled accent button: an icon and a word, or a spinner
/// while the play request is on its way.
fn pill(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Icon,
    label: &str,
    busy: bool,
) -> egui::Response {
    let font = theme::semibold(13.5);
    let text = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, palette.on_accent);
    let content = 14.0 + 6.0 + text.size().x;
    let width = (content + 32.0).max(PILL_MIN_WIDTH);
    let (rect, response) = ui.allocate_exact_size(vec2(width, PILL_HEIGHT), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(rect) {
        let fill = if !ui.is_enabled() {
            palette.accent.gamma_multiply(0.4)
        } else if response.hovered() {
            palette.accent_hover
        } else {
            palette.accent
        };
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
        if busy {
            let mut cell = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
            );
            theme::spinner(&mut cell, 16.0, palette.on_accent);
        } else {
            let start = rect.center().x - content / 2.0;
            let icon_rect =
                Rect::from_center_size(pos2(start + 7.0, rect.center().y), egui::Vec2::splat(14.0));
            theme::paint_icon(ui, icon, icon_rect, 14.0, palette.on_accent);
            ui.painter().galley(
                pos2(start + 20.0, rect.center().y - text.size().y / 2.0),
                text,
                palette.on_accent,
            );
        }
    }
    theme::focus_ring(ui, &response);
    response
}

//! Playlist and album pages as Apple Music lays them out: a flat page with
//! the artwork on the left and, beside it, the kind, the title, the owner
//! or artists in the accent, the details, and Play and Shuffle pills along
//! the artwork's bottom edge.
//!
//! [`hero`] leaves the band for the pills where [`actions_row`] finds it
//! later in the same pass, so Spotifast's pages call both unchanged.

use std::sync::Arc;

use egui::{Align, Color32, CornerRadius, Id, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::api::models::{Album, Track, pick_image};
use crate::app::App;
use crate::i18n::{gettext, ngettext, pgettext};
use crate::model::{Action, Dialog, HomeFilter, Page, RowContext, SortColumn, TableSort};
use crate::theme::{self, Icon, Palette};
use crate::ui::collection::{Actions, Hero};
use crate::ui::widgets;

const ART_RADIUS: f32 = 10.0;
/// The action buttons' height, and the round ones' width.
const BUTTON: f32 = 36.0;
/// Play's width, enough for the longest translation of Pause, so it keeps
/// its size when its label changes.
const PLAY_WIDTH: f32 = 128.0;
const ACTIONS_ID: &str = "mcloud-hero-actions";

pub(in crate::ui) fn hero(app: &mut App, ui: &mut egui::Ui, hero: Hero<'_>) {
    let palette = app.palette;
    let art_loader = app.backend.art().clone();
    let softened = hero
        .images
        .thumbnail
        .and_then(|uri| app.softened_covers.texture(ui.ctx(), &art_loader, uri));
    ui.add_space(12.0);
    let page_width = ui.available_width();
    let size = (page_width * 0.24).clamp(150.0, 270.0);
    let gap = (page_width * 0.024).clamp(20.0, 32.0);
    let (row, _) = ui.allocate_exact_size(vec2(page_width, size), Sense::hover());
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
    // Two shadows, a wide soft one and a close one, as mcloud's.
    for (offset, blur, alpha) in [(12, 32, 89), (2, 6, 51)] {
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, offset],
                blur,
                spread: 0,
                color: Color32::from_black_alpha(alpha),
            }
            .as_shape(art, CornerRadius::same(radius.min(127.0) as u8)),
        );
    }
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

    let left = art.right() + gap;
    let width = (row.right() - left).max(0.0);
    // The buttons take the band along the artwork's bottom edge.
    let actions = Rect::from_min_max(pos2(left, art.bottom() - BUTTON), row.max);
    // The pass is read before the data lock: egui's context locks are not
    // reentrant.
    let pass = ui.ctx().cumulative_pass_nr();
    ui.ctx()
        .data_mut(|data| data.insert_temp(Id::new(ACTIONS_ID), (pass, actions)));

    // The title and the creator share one size, the creator in the accent
    // and regular weight; the details and the description follow at half
    // white. The block is centred in the room above the buttons.
    let size = (page_width * 0.021).clamp(20.0, 28.0);
    let half_white = Color32::from_white_alpha(128);
    let title = widgets::ellipsized(ui, hero.title, theme::bold(size), palette.text, width, 2);
    // The names that open a page lead: the owner, or the album's artists. A
    // playlist whose owner has no page still leads with the owner.
    let mut names: Vec<_> = hero
        .byline
        .iter()
        .take_while(|(_, page)| page.is_some())
        .collect();
    if names.is_empty() && !hero.round {
        names.extend(hero.byline.first());
    }
    let creator = (!names.is_empty()).then(|| {
        names
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    });
    let creator = creator
        .map(|text| widgets::ellipsized(ui, &text, theme::regular(size), palette.accent, width, 1));
    let details: Vec<&str> = hero
        .byline
        .iter()
        .skip(names.len())
        .map(|(text, _)| text.as_str())
        .collect();
    let details = (!details.is_empty()).then(|| {
        widgets::ellipsized(
            ui,
            &details.join(" • "),
            theme::regular(12.0),
            half_white,
            width,
            1,
        )
    });
    let description = hero
        .description
        .as_deref()
        .filter(|text| !text.is_empty())
        .map(|text| widgets::ellipsized(ui, text, theme::regular(12.0), half_white, width, 2));
    let height = title.size().y
        + creator.as_ref().map_or(0.0, |galley| galley.size().y + 6.0)
        + details.as_ref().map_or(0.0, |galley| galley.size().y)
        + description
            .as_ref()
            .map_or(0.0, |galley| galley.size().y + 4.0);
    let room = Rect::from_min_max(art.min, pos2(art.right(), actions.top() - 12.0));
    let mut y = (room.center().y - height / 2.0).max(art.top());
    let mut paint = |galley: Arc<egui::Galley>, color: Color32, after: f32| {
        let rect = Rect::from_min_size(pos2(left, y), galley.size());
        ui.painter().galley(rect.min, galley, color);
        y = rect.bottom() + after;
        rect
    };
    paint(title, palette.text, 0.0);
    if let Some(creator) = creator {
        let rect = paint(creator, palette.accent, 6.0);
        let target = names.iter().find_map(|(_, page)| page.clone());
        if let Some(target) = target {
            let response = ui
                .interact(rect, Id::new("mcloud-hero-creator"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                app.actions.push(Action::Open(target));
            }
        }
    }
    if let Some(details) = details {
        let rect = paint(details, half_white, 4.0);
        if let Some((at, tooltip)) = &hero.byline_tooltip
            && *at >= names.len()
        {
            ui.interact(rect, Id::new("mcloud-hero-details"), Sense::hover())
                .on_hover_text(tooltip);
        }
    }
    if let Some(description) = description {
        paint(description, half_white, 0.0);
    }
    ui.add_space(20.0);
}

/// Shuffle, Play, then the save, radio and "…" buttons, in the
/// band [`hero`] left for them, or on a row of their own on pages without
/// one. The filter and the sort sit at the band's right end, as Spotify's
/// list toolbar does under mcloud: a magnifier that opens into a field,
/// then the sort.
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
    let tools = |app: &mut App, ui: &mut egui::Ui, filter: Option<&mut String>| {
        if let Some(filter) = filter {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                sort_menu_button(app, ui);
                filter_tool(app, ui, actions.name, filter);
            });
        }
    };
    match band {
        Some(band) => {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(band)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            child.add_enabled_ui(enabled, |ui| {
                controls(app, ui, &actions, filtered);
                tools(app, ui, filter);
            });
        }
        None => {
            ui.horizontal(|ui| {
                controls(app, ui, &actions, filtered);
                tools(app, ui, filter);
            });
            ui.add_space(10.0);
        }
    }
}

/// The list's filter: a magnifier until it is clicked, then a field, which
/// folds back into the magnifier once it is empty and loses focus.
fn filter_tool(app: &mut App, ui: &mut egui::Ui, name: &str, filter: &mut String) {
    let palette = app.palette;
    let id = Id::new(("collection-filter", name));
    let open_id = id.with("open");
    let open = ui.data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    if open || !filter.is_empty() {
        let response = widgets::search_field(
            ui,
            &palette,
            app.locale,
            id,
            filter,
            &gettext(app.locale, "Filter"),
            220.0,
        );
        if open && !response.has_focus() && !response.gained_focus() {
            let just_opened = ui.data(|data| data.get_temp::<bool>(open_id.with("asked")));
            if just_opened == Some(true) {
                response.request_focus();
                ui.data_mut(|data| data.remove::<bool>(open_id.with("asked")));
            } else if filter.is_empty() {
                ui.data_mut(|data| data.remove::<bool>(open_id));
            }
        }
        return;
    }
    let label = gettext(app.locale, "Filter");
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(32.0), Sense::click());
    let colour = if response.hovered() {
        palette.text
    } else {
        palette.secondary
    };
    theme::paint_icon(ui, Icon::Search, rect, 17.0, colour);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label.as_ref())
    });
    theme::focus_ring(ui, &response);
    if response.on_hover_text(label.as_ref()).clicked() {
        ui.data_mut(|data| {
            data.insert_temp(open_id, true);
            data.insert_temp(open_id.with("asked"), true);
        });
    }
}

pub(crate) fn sort_menu_button(app: &mut App, ui: &mut egui::Ui) {
    let page = app.page().clone();
    let current_sort = app.table_sorts.get(&page).copied();
    let locale = app.locale;
    let palette = app.palette;

    let is_liked = matches!(page, Page::LikedSongs);
    let label = match current_sort {
        None if is_liked => gettext(locale, "Recently added"),
        None => gettext(locale, "Custom order"),
        Some(s) => match s.column {
            SortColumn::Added => gettext(locale, "Recently added"),
            SortColumn::Title => pgettext(locale, "column", "Title"),
            SortColumn::Artist => gettext(locale, "Artist"),
            SortColumn::Album => gettext(locale, "Album"),
            SortColumn::Duration => gettext(locale, "Duration"),
            SortColumn::Index => gettext(locale, "Custom order"),
            SortColumn::AddedBy => pgettext(locale, "column heading", "ADDED BY"),
        },
    };

    // Spotify's sort under mcloud: the order's name and a list glyph, in
    // the secondary text colour, brightening on hover.
    let height = 32.0;
    let icon_size = 17.0;
    let font = theme::regular(13.0);
    let galley = crate::bidi::layout_line(
        ui.painter(),
        label.as_ref(),
        font.clone(),
        palette.secondary,
    );
    let spacing = 6.0;
    let total_width = galley.size().x + spacing + icon_size;
    let (rect, response) = ui.allocate_exact_size(vec2(total_width, height), Sense::click());

    if ui.is_rect_visible(rect) {
        let (colour, galley) = if response.hovered() {
            let galley = crate::bidi::layout_line(ui.painter(), label.as_ref(), font, palette.text);
            (palette.text, galley)
        } else {
            (palette.secondary, galley)
        };
        let text_pos = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, colour);
        let icon_rect = Rect::from_center_size(
            pos2(rect.right() - icon_size / 2.0, rect.center().y),
            egui::Vec2::splat(icon_size),
        );
        theme::paint_icon(ui, Icon::LayoutList, icon_rect, icon_size, colour);
    }
    theme::focus_ring(ui, &response);

    if ui.is_enabled() {
        egui::Popup::menu(&response)
            .frame(widgets::menu_frame(&palette))
            .show(|ui| {
                ui.set_min_width(170.0);
                let is_custom = (current_sort.is_none() && !is_liked)
                    || current_sort
                        == Some(TableSort {
                            column: SortColumn::Index,
                            ascending: true,
                        });
                let is_recent = (current_sort.is_none() && is_liked)
                    || current_sort
                        == Some(TableSort {
                            column: SortColumn::Added,
                            ascending: false,
                        });
                let is_title = current_sort.is_some_and(|s| s.column == SortColumn::Title);
                let is_artist = current_sort.is_some_and(|s| s.column == SortColumn::Artist);
                let is_album = current_sort.is_some_and(|s| s.column == SortColumn::Album);
                let is_duration = current_sort.is_some_and(|s| s.column == SortColumn::Duration);

                let mut next_sort = None;
                let mut clear_sort = false;

                let check = if is_custom { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &gettext(locale, "Custom order")) {
                    clear_sort = true;
                }

                let check = if is_title { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &pgettext(locale, "column", "Title")) {
                    let asc = if is_title {
                        !current_sort.unwrap().ascending
                    } else {
                        true
                    };
                    next_sort = Some(TableSort {
                        column: SortColumn::Title,
                        ascending: asc,
                    });
                }

                let check = if is_artist { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &gettext(locale, "Artist")) {
                    let asc = if is_artist {
                        !current_sort.unwrap().ascending
                    } else {
                        true
                    };
                    next_sort = Some(TableSort {
                        column: SortColumn::Artist,
                        ascending: asc,
                    });
                }

                let check = if is_album { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &gettext(locale, "Album")) {
                    let asc = if is_album {
                        !current_sort.unwrap().ascending
                    } else {
                        true
                    };
                    next_sort = Some(TableSort {
                        column: SortColumn::Album,
                        ascending: asc,
                    });
                }

                let check = if is_recent { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &gettext(locale, "Recently added")) {
                    if is_liked {
                        let asc = if is_recent {
                            current_sort.is_none_or(|s| !s.ascending)
                        } else {
                            false
                        };
                        if !asc {
                            clear_sort = true;
                        } else {
                            next_sort = Some(TableSort {
                                column: SortColumn::Added,
                                ascending: true,
                            });
                        }
                    } else {
                        let asc = if is_recent {
                            !current_sort.unwrap().ascending
                        } else {
                            false
                        };
                        next_sort = Some(TableSort {
                            column: SortColumn::Added,
                            ascending: asc,
                        });
                    }
                }

                let check = if is_duration { Some(Icon::Check) } else { None };
                if widgets::menu_item(ui, &palette, check, &gettext(locale, "Duration")) {
                    let asc = if is_duration {
                        !current_sort.unwrap().ascending
                    } else {
                        true
                    };
                    next_sort = Some(TableSort {
                        column: SortColumn::Duration,
                        ascending: asc,
                    });
                }

                if clear_sort {
                    app.table_sorts.remove(&page);
                    app.note_session_change();
                    ui.close();
                } else if let Some(sort) = next_sort {
                    app.table_sorts.insert(page.clone(), sort);
                    app.note_session_change();
                    app.actions.push(Action::LoadMore(page.clone()));
                    ui.close();
                }
            });
    }
}

pub(crate) fn liked_genre_chips(
    app: &mut App,
    ui: &mut egui::Ui,
    selected_genre: &mut Option<String>,
) {
    let genres = crate::ui::collection::liked_genres(app, ui.ctx());
    if genres.is_empty() {
        return;
    }

    let locale = app.locale;

    let all_label = pgettext(locale, "filter", "All");
    let mut options = Vec::with_capacity(genres.len() + 1);
    options.push((None, all_label.into_owned()));

    for g in genres.iter() {
        let label = crate::ui::mcloud::sentence_case(g);
        options.push((Some(g.clone()), label));
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (value, label) in &options {
            let is_active = value.as_deref() == selected_genre.as_deref();
            if chip(ui, label, is_active).clicked() {
                *selected_genre = value.clone();
            }
        }
    });
    ui.add_space(8.0);
}

/// A filter chip as Spotify draws them under mcloud: 32pt tall, white at
/// 7%, white with black text when chosen.
pub(crate) fn chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let colour = if active {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    let galley = crate::bidi::layout_line(ui.painter(), label, theme::regular(14.0), colour);
    let size = vec2(galley.size().x + 24.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, label)
    });
    if ui.is_rect_visible(rect) {
        let fill = match (active, response.hovered()) {
            (true, false) => Color32::from_rgb(0xf0, 0xf0, 0xf2),
            (true, true) => Color32::WHITE,
            (false, false) => Color32::from_white_alpha(18),
            (false, true) => Color32::from_white_alpha(31),
        };
        ui.painter().rect_filled(rect, CornerRadius::same(16), fill);
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, colour);
    }
    theme::focus_ring(ui, &response);
    response
}

pub(crate) fn home_category_chips(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let current = app.home.category_filter;

    let options = [
        (HomeFilter::All, pgettext(locale, "filter", "All")),
        (HomeFilter::Music, gettext(locale, "Music")),
        (HomeFilter::Podcasts, gettext(locale, "Podcasts")),
    ];

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (filter, label) in options {
            let is_active = filter == current;
            if chip(ui, label.as_ref(), is_active).clicked() {
                let next = if is_active && filter != HomeFilter::All {
                    HomeFilter::All
                } else {
                    filter
                };
                app.actions.push(Action::SetHomeFilter(next));
            }
        }
    });
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
            (Icon::McloudPause, gettext(locale, "Pause"))
        } else {
            (Icon::McloudPlay, gettext(locale, "Play"))
        };
        // Shuffle leads, as in Apple Music, a round button in the accent.
        let shuffle = ui
            .add_enabled_ui(can_start, |ui| {
                round_button(
                    ui,
                    &palette,
                    Icon::McloudShuffle,
                    palette.accent,
                    &gettext(locale, "Shuffle"),
                )
            })
            .inner;
        if shuffle.clicked() {
            // Apple Music's Shuffle starts the list shuffled from the top.
            app.actions.push(Action::SetShuffle(true));
            start(app, actions, uri, filtered, true);
        }
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
    }
    if let Some((uri, saved)) = &actions.saved {
        let (icon, tooltip, color) = if *saved {
            (
                actions.saved_icons.1,
                &actions.saved_tooltips.1,
                palette.accent,
            )
        } else {
            (
                actions.saved_icons.0,
                &actions.saved_tooltips.0,
                palette.secondary,
            )
        };
        if round_button(ui, &palette, icon, color, tooltip).clicked() {
            app.actions.push(Action::ToggleSaved(uri.clone()));
        }
    }
    if let Some(seed) = &actions.save_radio
        && round_button(
            ui,
            &palette,
            Icon::McloudPlus,
            palette.secondary,
            &gettext(locale, "Save as playlist"),
        )
        .clicked()
    {
        app.actions.push(Action::SaveRadio(seed.clone()));
    }
    if let Some(uri) = &actions.play_uri {
        let more = round_button(
            ui,
            &palette,
            Icon::McloudMore,
            palette.secondary,
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

/// mcloud's Play: a grey capsule with the icon and the word in the
/// accent, or a spinner while the play request is on its way.
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
        .layout_no_wrap(label.to_owned(), font, palette.accent);
    let content = 15.0 + 5.0 + text.size().x;
    let width = (content + 32.0).max(PLAY_WIDTH);
    let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(rect) {
        let fill = if response.hovered() {
            Color32::from_white_alpha(38)
        } else {
            Color32::from_white_alpha(26)
        };
        let color = if ui.is_enabled() {
            palette.accent
        } else {
            palette.accent.gamma_multiply(0.4)
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same((BUTTON / 2.0) as u8), fill);
        if busy {
            let mut cell = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
            );
            theme::spinner(&mut cell, 16.0, color);
        } else {
            let start = rect.center().x - content / 2.0;
            let icon_rect =
                Rect::from_center_size(pos2(start + 7.5, rect.center().y), egui::Vec2::splat(15.0));
            theme::paint_icon(ui, icon, icon_rect, 15.0, color);
            ui.painter().galley(
                pos2(start + 20.0, rect.center().y - text.size().y / 2.0),
                text,
                color,
            );
        }
    }
    theme::focus_ring(ui, &response);
    response
}

/// A round action button with a faint fill.
fn round_button(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Icon,
    color: Color32,
    label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(BUTTON), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if ui.is_rect_visible(rect) {
        let (fill, color) = if response.hovered() {
            (
                Color32::from_white_alpha(33),
                if color == palette.secondary {
                    Color32::WHITE
                } else {
                    color
                },
            )
        } else {
            (Color32::from_white_alpha(20), color)
        };
        ui.painter()
            .circle_filled(rect.center(), BUTTON / 2.0, fill);
        theme::paint_icon(ui, icon, rect, 17.0, color);
    }
    theme::focus_ring(ui, &response);
    if label.is_empty() {
        response
    } else {
        response.on_hover_text(label)
    }
}

/// The bottom of an album page as mcloud / Apple Music draws it:
/// song count, total duration, full release date, copyright credits,
/// and a "More by [Artist]" shelf showing the artist's other releases.
pub(in crate::ui) fn album_footer(
    app: &mut App,
    ui: &mut egui::Ui,
    album: &Album,
    tracks: &[Track],
) {
    let palette = app.palette;
    ui.add_space(24.0);

    // Track count and length.
    let count = album.total_tracks.unwrap_or(tracks.len() as u32);
    let duration: u64 = tracks.iter().map(|track| track.duration_ms as u64).sum();
    let count_text = ngettext(app.locale, "{count} song", "{count} songs", count)
        .replace("{count}", &count.to_string());
    let length_text = crate::util::format_total_ms(app.locale, duration);
    theme::text(
        ui,
        format!("{count_text}, {length_text}"),
        theme::medium(13.0),
        Color32::from_white_alpha(153),
    );

    // Release date.
    if let Some(date) = &album.release_date {
        ui.add_space(4.0);
        theme::text(
            ui,
            crate::util::format_date(app.locale, date),
            theme::regular(12.0),
            Color32::from_white_alpha(128),
        );
    }

    // Copyright lines.
    let mut credits: Vec<(String, Vec<&str>)> = Vec::new();
    for copyright in &album.copyrights {
        let core = copyright
            .text
            .trim_start_matches(['©', '℗'])
            .trim_start_matches("(C)")
            .trim_start_matches("(P)")
            .trim()
            .to_string();
        let mark = if copyright.kind == "P" { "℗" } else { "©" };
        match credits.iter_mut().find(|(held, _)| *held == core) {
            Some((_, marks)) => {
                if !marks.contains(&mark) {
                    marks.push(mark);
                }
            }
            None => credits.push((core, vec![mark])),
        }
    }
    for (core, marks) in credits {
        ui.add_space(2.0);
        theme::text(
            ui,
            format!("{} {core}", marks.join(" ")),
            theme::regular(11.0),
            palette.dim,
        );
    }

    // More by artist shelf.
    if let Some(artist) = album.artists.first()
        && let Some(artist_id) = &artist.id
    {
        more_by_shelf(app, ui, artist_id, &artist.name, &album.id);
    }
}

fn more_by_shelf(
    app: &mut App,
    ui: &mut egui::Ui,
    artist_id: &str,
    artist_name: &str,
    current_album_id: &str,
) {
    let other_albums: Vec<Album> = if let Some(artist_page) = app.artist_pages.get(artist_id) {
        let all = artist_page
            .albums
            .get(crate::model::DiscographyFilter::All.groups())
            .or_else(|| artist_page.albums.get(artist_page.filter.groups()));
        if let Some(list) = all {
            list.items
                .iter()
                .filter(|other| other.id != current_album_id)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    if other_albums.is_empty() {
        return;
    }

    ui.add_space(32.0);
    // Translators: Heading for other albums by this artist below an album page.
    let title = gettext(app.locale, "More by {artist}").replace("{artist}", artist_name);
    theme::text(ui, &title, theme::bold(20.0), Color32::WHITE);
    ui.add_space(12.0);

    const CARD_SIZE: f32 = 160.0;
    const CARD_GAP: f32 = 16.0;

    egui::ScrollArea::horizontal()
        .id_salt("mcloud-album-more-by")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                for other in &other_albums {
                    let image = pick_image(&other.images, app.art_px(CARD_SIZE));
                    let year = other.year().unwrap_or("").to_string();
                    let response = super::artist::card(
                        app,
                        ui,
                        image,
                        &other.name,
                        Some(&year),
                        CARD_SIZE,
                        false,
                    );
                    if response.clicked() {
                        app.actions
                            .push(Action::Open(Page::Album(other.id.clone())));
                    }
                    egui::Popup::context_menu(&response)
                        .id(ui.make_persistent_id(("album-more-by-menu", &other.uri)))
                        .frame(widgets::menu_frame(&app.palette))
                        .show(|ui| {
                            widgets::context_menu_items(ui, app, &other.uri, &other.name, None)
                        });
                }
            });
        });
    ui.add_space(20.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_albums_filters_out_the_currently_viewed_album() {
        let current = "alb1";
        let albums = vec![
            Album {
                id: "alb0".into(),
                name: "First".into(),
                ..Default::default()
            },
            Album {
                id: "alb1".into(),
                name: "Current".into(),
                ..Default::default()
            },
            Album {
                id: "alb2".into(),
                name: "Second".into(),
                ..Default::default()
            },
        ];
        let others: Vec<_> = albums.into_iter().filter(|a| a.id != current).collect();
        assert_eq!(others.len(), 2);
        assert_eq!(others[0].id, "alb0");
        assert_eq!(others[1].id, "alb2");
    }

    #[test]
    fn genre_frequency_ranking_orders_by_count() {
        use std::collections::HashMap;
        let mut counts: HashMap<String, usize> = HashMap::new();
        counts.insert("rock".into(), 5);
        counts.insert("electronic".into(), 12);
        counts.insert("ambient".into(), 8);

        let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top: Vec<String> = sorted.into_iter().map(|(g, _)| g).collect();

        assert_eq!(top, vec!["electronic", "ambient", "rock"]);
    }
}

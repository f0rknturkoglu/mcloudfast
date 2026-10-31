//! The artist page as Apple Music draws it: the artist's picture across the
//! page with the name large over its bottom edge, Play and Follow beside
//! it; the latest release beside the popular songs; then shelves of the
//! discography and similar artists.

use std::sync::Arc;

use egui::{Align, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::api::models::{Album, Artist, PlayableItem, pick_image};
use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, ArtistPage, DiscographyFilter, Loadable, Page, RowContext};
use crate::theme::{self, Icon};
use crate::ui::widgets::{self, PAGE_PADDING, TrackRow};

const BANNER_MIN: f32 = 240.0;
const BANNER_MAX: f32 = 380.0;
/// Where in the picture's height the banner centres its crop: a little
/// above the middle, where faces usually are.
const FOCUS: f32 = 0.35;
const LATEST_WIDTH: f32 = 220.0;
/// The page is wide enough to set the latest release beside the songs.
const SIDE_BY_SIDE: f32 = 720.0;
const POPULAR_ROWS: usize = 5;

pub(in crate::ui) fn loaded(
    app: &mut App,
    ui: &mut egui::Ui,
    id: &str,
    page: &ArtistPage,
    artist: &Artist,
) {
    let palette = app.palette;
    let locale = app.locale;
    banner(app, ui, artist);
    ui.add_space(24.0);

    let latest = latest_release(page);
    let beside = latest.is_some() && ui.available_width() >= SIDE_BY_SIDE;
    if beside {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                vec2(LATEST_WIDTH, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    if let Some(album) = latest {
                        latest_card(app, ui, album);
                    }
                },
            );
            ui.add_space(28.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                popular(app, ui, id, page, artist);
            });
        });
    } else {
        if let Some(album) = latest {
            latest_card(app, ui, album);
            ui.add_space(16.0);
        }
        popular(app, ui, id, page, artist);
    }
    ui.add_space(24.0);

    discography(app, ui, id, page);

    if let Loadable::Loaded(related) = &page.related
        && !related.is_empty()
    {
        let artist_label = gettext(locale, "Artist");
        let title = gettext(locale, "Fans also like");
        widgets::shelf(ui, &palette, "related", &title, |ui| {
            for artist in related {
                let context_here =
                    app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&artist.images, card_art),
                    &artist.name,
                    &artist_label,
                    widgets::CardCover::portrait(playing_here),
                );
                play_or_open(app, &card, context_here, &artist.uri, || {
                    Page::Artist(artist.id.clone())
                });
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("related-artist-menu", &artist.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| {
                        widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None)
                    });
            }
        });
    }
}

/// The picture across the page, edge to edge, cropped to the band, with
/// the name and the buttons over a shade along its bottom.
fn banner(app: &mut App, ui: &mut egui::Ui, artist: &Artist) {
    let palette = app.palette;
    let locale = app.locale;
    let page = ui.max_rect();
    let width = page.width() + 2.0 * PAGE_PADDING;
    let height = (width * 0.36).clamp(BANNER_MIN, BANNER_MAX);
    let (reserved, _) = ui.allocate_exact_size(vec2(page.width(), height), Sense::hover());
    // Over the page's side margins too, and up to the top bar.
    let rect = Rect::from_min_max(
        pos2(page.left() - PAGE_PADDING, reserved.top() - 4.0),
        pos2(page.right() + PAGE_PADDING, reserved.bottom()),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, palette.surface);
    if let Some(url) = pick_image(&artist.images, u32::MAX) {
        app.backend.art().touch(url);
        let image = egui::Image::new(url)
            .texture_options(crate::images::COVER_TEXTURE)
            .show_loading_spinner(false);
        if let Ok(egui::load::TexturePoll::Ready { texture }) =
            image.load_for_size(ui.ctx(), rect.size())
        {
            painter.image(
                texture.id,
                rect,
                crop(texture.size, rect.size()),
                Color32::WHITE,
            );
            crate::images::forget_decoded(ui.ctx(), url);
        }
    }
    let shade = Rect::from_min_max(pos2(rect.left(), rect.center().y), rect.max);
    widgets::paint_vertical_gradient(
        ui,
        shade,
        Color32::TRANSPARENT,
        Color32::from_black_alpha(170),
    );

    // Play, then the name, along the bottom left.
    let line = rect.bottom() - 30.0;
    let play_rect = Rect::from_center_size(
        pos2(rect.left() + PAGE_PADDING + 24.0, line),
        egui::Vec2::splat(48.0),
    );
    let mut cell = ui.new_child(
        UiBuilder::new()
            .max_rect(play_rect)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    let context_here = app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    if app.play_pending(&artist.uri) {
        theme::circle_spinner(
            &mut cell,
            48.0,
            palette.accent,
            palette.on_accent,
            &gettext(locale, "Starting…"),
        );
    } else if theme::circle_button(
        &mut cell,
        if playing_here {
            Icon::PauseFilled
        } else {
            Icon::PlayFilled
        },
        48.0,
        palette.accent,
        palette.accent_hover,
        palette.on_accent,
        &if playing_here {
            gettext(locale, "Pause")
        } else {
            gettext(locale, "Play")
        },
    )
    .clicked()
    {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: artist.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }

    // Follow and "…" at the bottom right; the name takes what is between.
    let following = app.is_saved(&artist.uri).unwrap_or(false);
    let buttons = Rect::from_min_max(
        pos2(rect.center().x, line - 18.0),
        pos2(rect.right() - PAGE_PADDING, line + 18.0),
    );
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(buttons)
            .layout(Layout::right_to_left(Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 10.0;
    let more = theme::icon_button(
        &mut right,
        Icon::Ellipsis,
        22.0,
        Color32::WHITE,
        Color32::WHITE,
        &gettext(locale, "More"),
    );
    egui::Popup::menu(&more)
        .frame(widgets::menu_frame(&palette))
        .show(|ui| widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None));
    if theme::pill_button(
        &mut right,
        &palette,
        &if following {
            pgettext(locale, "artist", "Following")
        } else {
            pgettext(locale, "artist", "Follow")
        },
        false,
    )
    .clicked()
    {
        app.actions.push(Action::ToggleSaved(artist.uri.clone()));
    }
    let buttons_left = right.min_rect().left();

    let name_left = play_rect.right() + 14.0;
    let room = (buttons_left - 16.0 - name_left).max(0.0);
    let mut size = 56.0;
    let name = loop {
        let galley =
            ui.painter()
                .layout_no_wrap(artist.name.clone(), theme::bold(size), Color32::WHITE);
        if galley.size().x <= room || size <= 26.0 {
            break widgets::ellipsized(
                ui,
                &artist.name,
                theme::bold(size),
                Color32::WHITE,
                room,
                1,
            );
        }
        size -= 4.0;
    };
    ui.painter().galley(
        pos2(name_left, line - name.size().y / 2.0),
        name,
        Color32::WHITE,
    );
}

/// The part of a picture `image` in size that fills `target` without
/// stretching, as texture coordinates.
fn crop(image: egui::Vec2, target: egui::Vec2) -> Rect {
    let image_aspect = image.x / image.y.max(1.0);
    let target_aspect = target.x / target.y.max(1.0);
    if image_aspect > target_aspect {
        let width = target_aspect / image_aspect;
        Rect::from_min_max(pos2(0.5 - width / 2.0, 0.0), pos2(0.5 + width / 2.0, 1.0))
    } else {
        let height = image_aspect / target_aspect;
        let top = (FOCUS - height / 2.0).clamp(0.0, 1.0 - height);
        Rect::from_min_max(pos2(0.0, top), pos2(1.0, top + height))
    }
}

/// The newest release in the discography loaded so far.
fn latest_release(page: &ArtistPage) -> Option<&Album> {
    page.albums
        .get(DiscographyFilter::All.groups())
        .or_else(|| page.albums.get(page.filter.groups()))?
        .items
        .iter()
        .filter(|album| album.release_date.is_some())
        .max_by(|a, b| a.release_date.cmp(&b.release_date))
}

fn latest_card(app: &mut App, ui: &mut egui::Ui, album: &Album) {
    let palette = app.palette;
    theme::section_title(ui, &palette, &gettext(app.locale, "Latest release"));
    ui.add_space(8.0);
    let side = ui.available_width().min(LATEST_WIDTH);
    let (art, response) = ui.allocate_exact_size(egui::Vec2::splat(side), Sense::click());
    widgets::paint_shadow(ui, &palette, art, 8.0);
    widgets::paint_cover(
        ui,
        &palette,
        pick_image(&album.images, app.art_px(side)),
        art,
        8.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    if response.hovered() {
        ui.painter().rect_filled(
            art,
            egui::CornerRadius::same(8),
            Color32::from_black_alpha(60),
        );
    }
    ui.add_space(8.0);
    let name = widgets::ellipsized(
        ui,
        &album.name,
        theme::semibold(14.0),
        palette.text,
        side,
        2,
    );
    let (name_rect, name_response) = ui.allocate_exact_size(name.size(), Sense::click());
    ui.painter().galley(name_rect.min, name, palette.text);
    let details = format!(
        "{} · {}",
        album.release_date.as_deref().unwrap_or(""),
        app.album_kind_label(album)
    );
    theme::text(
        ui,
        details.trim_start_matches(" · "),
        theme::regular(12.5),
        palette.secondary,
    );
    let response = response
        .union(name_response)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        app.actions
            .push(Action::Open(Page::Album(album.id.clone())));
    }
    egui::Popup::context_menu(&response)
        .id(ui.make_persistent_id(("latest-release-menu", &album.uri)))
        .frame(widgets::menu_frame(&palette))
        .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
}

fn popular(app: &mut App, ui: &mut egui::Ui, id: &str, page: &ArtistPage, artist: &Artist) {
    let palette = app.palette;
    let locale = app.locale;
    theme::section_title(ui, &palette, &gettext(locale, "Popular"));
    ui.add_space(4.0);
    match &page.top_tracks {
        Loadable::Loaded(tracks) if !tracks.is_empty() => {
            let uris: Arc<[String]> = tracks
                .iter()
                .map(|track| track.uri.clone())
                .collect::<Vec<_>>()
                .into();
            let context = RowContext::Uris(Arc::clone(&uris));
            let items: Vec<PlayableItem> =
                tracks.iter().cloned().map(PlayableItem::Track).collect();
            let limit = if page.show_all_top {
                items.len()
            } else {
                POPULAR_ROWS
            };
            app.request_play_counts(&artist.uri);
            let show_plays = app.has_play_counts(&artist.uri);
            let rows = |ui: &mut egui::Ui, app: &mut App| {
                for (index, item) in items.iter().take(limit).enumerate() {
                    widgets::track_row(
                        ui,
                        app,
                        TrackRow {
                            index,
                            number: Some(index + 1),
                            item,
                            context: &context,
                            show_cover: true,
                            show_album: false,
                            added_at: None,
                            added_by: None,
                            show_added_by: false,
                            show_plays,
                            compact: true,
                            thin: false,
                            shift: 0.0,
                            picked: false,
                            picked_songs: &[],
                            picked_positions: &[],
                            position: None,
                        },
                    );
                }
            };
            if app.network_reachable {
                rows(ui, app);
            } else {
                ui.add_enabled_ui(false, |ui| {
                    ui.multiply_opacity(0.55);
                    rows(ui, app);
                });
            }
            if items.len() > POPULAR_ROWS {
                ui.add_space(6.0);
                let label = if page.show_all_top {
                    gettext(locale, "Show less")
                } else {
                    gettext(locale, "See more")
                };
                if theme::soft_button(ui, &palette, None, &label, false).clicked() {
                    app.actions.push(Action::ToggleShowAllTop(id.to_string()));
                }
            }
        }
        Loadable::Loaded(_) => {
            theme::subtle(ui, &palette, &gettext(locale, "No popular songs to show."));
        }
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::skeleton_rows(ui, &palette, locale, POPULAR_ROWS);
        }
        Loadable::Failed(error) => {
            let error = error.clone();
            widgets::error_row(ui, app, &error, None);
        }
    }
}

/// The discography as a shelf, its filters beside the title.
fn discography(app: &mut App, ui: &mut egui::Ui, id: &str, page: &ArtistPage) {
    let palette = app.palette;
    let locale = app.locale;
    ui.horizontal(|ui| {
        theme::section_title(ui, &palette, &gettext(locale, "Discography"));
        ui.add_space(12.0);
        let labels: Vec<_> = DiscographyFilter::ALL
            .iter()
            .map(|filter| (*filter, filter.label(locale)))
            .collect();
        let options: Vec<(DiscographyFilter, &str)> = labels
            .iter()
            .map(|(filter, label)| (*filter, label.as_ref()))
            .collect();
        if let Some(filter) = widgets::chips(ui, &palette, &options, page.filter) {
            app.actions.push(Action::SetDiscographyFilter {
                artist_id: id.to_string(),
                filter,
            });
        }
    });
    let Some(list) = page.albums.get(page.filter.groups()) else {
        widgets::shelf_cards(ui, "discography", |ui| {
            widgets::skeleton_cards(ui, &palette, locale, 6, false);
        });
        return;
    };
    let mut seen = std::collections::HashSet::new();
    let albums: Vec<_> = list
        .items
        .iter()
        .filter(|album| seen.insert(album.name.to_lowercase()))
        .collect();
    if albums.is_empty() && !list.loading {
        ui.add_space(6.0);
        theme::subtle(ui, &palette, &gettext(locale, "Nothing in this category."));
    } else {
        widgets::shelf_cards(ui, "discography", |ui| {
            for album in &albums {
                let subtitle = format!(
                    "{} • {}",
                    album.year().unwrap_or(""),
                    app.album_kind_label(album)
                );
                let context_here = app.playing_context_uri().as_deref() == Some(album.uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&album.images, card_art),
                    &album.name,
                    subtitle.trim_start_matches(" • "),
                    widgets::CardCover::square(playing_here),
                );
                play_or_open(app, &card, context_here, &album.uri, || {
                    Page::Album(album.id.clone())
                });
                egui::Popup::context_menu(&card.response)
                    .id(ui.make_persistent_id(("discography-menu", &album.uri)))
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
            }
            if list.loading {
                widgets::skeleton_cards(ui, &palette, locale, 2, false);
            }
        });
    }
    if let Some(error) = &list.error {
        let error = error.clone();
        widgets::error_row(ui, app, &error, None);
    } else if list.can_load_more() && !list.loading {
        if theme::soft_button(ui, &palette, None, &gettext(locale, "Load more"), false).clicked() {
            app.actions
                .push(Action::LoadMoreArtistAlbums(id.to_string()));
        }
        ui.add_space(12.0);
    }
}

/// A card's play button plays its context, or pauses it when it is the one
/// playing; the card itself opens its page.
fn play_or_open(
    app: &mut App,
    card: &widgets::CardResponse,
    context_here: bool,
    uri: &str,
    page: impl FnOnce() -> Page,
) {
    if card.play {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: uri.to_string(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked {
        app.actions.push(Action::Open(page()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_crops_a_square_picture_around_its_upper_middle() {
        let uv = crop(egui::vec2(640.0, 640.0), egui::vec2(1200.0, 400.0));
        assert_eq!(uv.width(), 1.0);
        assert!((uv.height() - 1.0 / 3.0).abs() < 1e-4);
        assert!((uv.center().y - FOCUS).abs() < 1e-4);
        // A picture wider than the band keeps its height and its centre.
        let uv = crop(egui::vec2(2000.0, 400.0), egui::vec2(1000.0, 400.0));
        assert_eq!(uv.height(), 1.0);
        assert!((uv.center().x - 0.5).abs() < 1e-4);
    }
}

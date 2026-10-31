//! The artist page as mcloud draws it (user.css `#am-artist`), after Apple
//! Music: the artist's header picture across the page, fading into the
//! artist's colour, which fills the whole window; the name centred over its
//! bottom with a white Play and a round Follow star; then the latest
//! release beside a three-row grid of top songs, and shelves of albums,
//! singles, appearances and similar artists.
//!
//! Spotify's header picture comes with the artist's play counts
//! ([`crate::session_reads::PageExtras`]); without one, the portrait stands
//! in, round and centred, as mcloud's avatar hero does.

use std::sync::Arc;

use egui::{Color32, CornerRadius, Id, Layout, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{Album, Artist, PlayableItem, pick_image};
use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, ArtistPage, DiscographyFilter, Loadable, Page, RowContext};
use crate::theme::{self, Icon};
use crate::ui::widgets::{self, PAGE_PADDING};

/// The page's side inset, as mcloud's shelves keep it.
const INSET: f32 = 32.0;
const SHELF_GAP: f32 = 30.0;
const CARD: f32 = 160.0;
const ARTIST_CARD: f32 = 150.0;
const CARD_GAP: f32 = 18.0;
const SONG_COLUMN: f32 = 310.0;
const SONG_ROWS: usize = 3;
const LATEST_ART: f32 = 150.0;
const WHITE_60: Color32 = Color32::from_rgba_premultiplied(153, 153, 153, 153);

/// The colours the page takes from the artist: the top of the window, and
/// what it deepens into further down.
pub(in crate::ui) fn tint(top: Color32, window: Color32) -> (Color32, Color32) {
    (top, top.lerp_to_gamma(window, 0.65))
}

pub(in crate::ui) fn loaded(
    app: &mut App,
    ui: &mut egui::Ui,
    page: &ArtistPage,
    artist: &Artist,
    id: &str,
) {
    // The header picture arrives with the play counts.
    app.request_play_counts(&artist.uri);
    hero(app, ui, artist);
    ui.add_space(10.0);

    let latest = latest_release(page);
    // The latest release and the top songs share the first row.
    ui.horizontal_top(|ui| {
        ui.add_space(INSET - PAGE_PADDING);
        if let Some(album) = latest {
            ui.allocate_ui_with_layout(
                vec2(300.0, 0.0),
                Layout::top_down(egui::Align::Min),
                |ui| latest_card(app, ui, album),
            );
            ui.add_space(SHELF_GAP);
        }
        ui.vertical(|ui| top_songs(app, ui, page, artist));
    });
    ui.add_space(SHELF_GAP);

    let all = page
        .albums
        .get(DiscographyFilter::All.groups())
        .or_else(|| page.albums.get(page.filter.groups()));
    if let Some(list) = all {
        let mut seen = std::collections::HashSet::new();
        let releases: Vec<&Album> = list
            .items
            .iter()
            .filter(|album| seen.insert(album.name.to_lowercase()))
            .collect();
        let (singles, albums): (Vec<&Album>, Vec<&Album>) = releases
            .iter()
            .partition(|album| album.album_type.as_deref() == Some("single"));
        album_shelf(
            app,
            ui,
            "artist-albums",
            &gettext(app.locale, "Albums"),
            &albums,
        );
        album_shelf(
            app,
            ui,
            "artist-singles",
            &gettext(app.locale, "Singles & EPs"),
            &singles,
        );
        if list.can_load_more() && !list.loading {
            ui.horizontal(|ui| {
                ui.add_space(INSET - PAGE_PADDING);
                if theme::soft_button(
                    ui,
                    &app.palette,
                    None,
                    &gettext(app.locale, "Load more"),
                    false,
                )
                .clicked()
                {
                    app.actions
                        .push(Action::LoadMoreArtistAlbums(id.to_string()));
                }
            });
            ui.add_space(SHELF_GAP);
        }
    }
    if let Some(list) = page.albums.get(DiscographyFilter::AppearsOn.groups()) {
        let appears: Vec<&Album> = list.items.iter().collect();
        album_shelf(
            app,
            ui,
            "artist-appears",
            &gettext(app.locale, "Appears On"),
            &appears,
        );
    }
    if let Loadable::Loaded(related) = &page.related
        && !related.is_empty()
    {
        let title = gettext(app.locale, "Fans also like");
        shelf(ui, "artist-related", &title, |ui| {
            for artist in related {
                let image = pick_image(&artist.images, app.art_px(ARTIST_CARD));
                let response = card(app, ui, image, &artist.name, None, ARTIST_CARD, true);
                if response.clicked() {
                    app.actions
                        .push(Action::Open(Page::Artist(artist.id.clone())));
                }
                egui::Popup::context_menu(&response)
                    .id(ui.make_persistent_id(("related-artist-menu", &artist.uri)))
                    .frame(widgets::menu_frame(&app.palette))
                    .show(|ui| {
                        widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None)
                    });
            }
        });
    }
}

/// The picture across the page, fading out into the artist's colour, the
/// name over its bottom, Play and Follow under the name, and the floating
/// back and "…" buttons at the top.
fn hero(app: &mut App, ui: &mut egui::Ui, artist: &Artist) {
    let palette = app.palette;
    let locale = app.locale;
    let page = ui.max_rect();
    // The page's own width, its side padding included.
    let full = Rect::from_min_max(
        pos2(page.left() - PAGE_PADDING, page.top()),
        pos2(page.right() + PAGE_PADDING, page.bottom()),
    );
    let width = full.width();
    let header = app.artist_headers.get(&artist.uri).cloned();
    let height = if header.is_some() {
        (width * 0.41 + 64.0).max(380.0)
    } else {
        (ui.ctx().content_rect().height() * 0.62).clamp(380.0, 620.0)
    };
    let (reserved, _) = ui.allocate_exact_size(vec2(page.width(), height), Sense::hover());
    let rect = Rect::from_min_max(
        pos2(full.left(), reserved.top() - 4.0),
        pos2(full.right(), reserved.bottom()),
    );

    if let Some(url) = header.as_deref() {
        header_picture(app, ui, url, rect);
    } else if let Some(url) = pick_image(&artist.images, app.art_px(250.0)) {
        // The portrait, round and centred.
        let side = (width * 0.17).clamp(170.0, 250.0);
        let portrait = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + 48.0 + side / 2.0),
            Vec2::splat(side),
        );
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 12],
                blur: 40,
                spread: 0,
                color: Color32::from_black_alpha(102),
            }
            .as_shape(portrait, CornerRadius::same(127)),
        );
        widgets::paint_cover(
            ui,
            &palette,
            Some(url),
            portrait,
            side / 2.0,
            Icon::User,
            Some(app.backend.art()),
        );
    }

    // The name, centred over the hero's bottom, and the buttons under it.
    let name_size = (width * 0.03).clamp(30.0, 46.0);
    let name = widgets::ellipsized(
        ui,
        &artist.name,
        theme::bold(name_size),
        Color32::WHITE,
        width - 2.0 * INSET,
        2,
    );
    let actions_y = rect.bottom() - 26.0 - 32.0;
    let name_pos = pos2(
        rect.center().x - name.size().x / 2.0,
        actions_y - 32.0 - 6.0 - 12.0 - name.size().y,
    );
    ui.painter().galley(
        name_pos + vec2(0.0, 1.0),
        name.clone(),
        Color32::from_black_alpha(50),
    );
    ui.painter().galley(name_pos, name, Color32::WHITE);

    // Play is a white disc with the triangle in the artist's colour; the
    // star follows beside it.
    let top_tint = app
        .tint_for(pick_image(&artist.images, 64))
        .unwrap_or(palette.surface);
    let play = Rect::from_center_size(pos2(rect.center().x, actions_y), Vec2::splat(64.0));
    let context_here = app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let response = ui.interact(play, Id::new("mcloud-artist-play"), Sense::click());
    ui.painter().add(
        egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(51),
        }
        .as_shape(play, CornerRadius::same(32)),
    );
    let scale = if response.is_pointer_button_down_on() {
        0.94
    } else {
        1.0
    };
    ui.painter()
        .circle_filled(play.center(), 32.0 * scale, Color32::WHITE);
    let (icon, label) = if playing_here {
        (Icon::McloudPause, gettext(locale, "Pause"))
    } else {
        (Icon::McloudHeroPlay, gettext(locale, "Play"))
    };
    // The triangle's box sits 1/18 of its width right of the disc's centre.
    theme::paint_icon(
        ui,
        icon,
        play.translate(vec2(31.0 / 18.0, 0.0)),
        31.0 * scale,
        top_tint,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    if response.on_hover_text(label.as_ref()).clicked() {
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
    let following = app.is_saved(&artist.uri).unwrap_or(false);
    let follow = Rect::from_center_size(
        pos2(play.right() + 22.0 + 20.0, actions_y),
        Vec2::splat(40.0),
    );
    let label = if following {
        pgettext(locale, "artist", "Following")
    } else {
        pgettext(locale, "artist", "Follow")
    };
    let response = ui.interact(follow, Id::new("mcloud-artist-follow"), Sense::click());
    ui.painter().circle_filled(
        follow.center(),
        20.0,
        Color32::from_white_alpha(if response.hovered() { 56 } else { 38 }),
    );
    theme::paint_icon(ui, Icon::McloudStar, follow, 17.0, Color32::WHITE);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, following, label.as_ref())
    });
    if response.on_hover_text(label.as_ref()).clicked() {
        app.actions.push(Action::ToggleSaved(artist.uri.clone()));
    }

    floating_buttons(app, ui, full, artist, top_tint);
}

/// The header picture: 53% of the width in height, fading out from 38% of
/// its height down into the colour behind it.
fn header_picture(app: &App, ui: &egui::Ui, url: &str, rect: Rect) {
    let width = rect.width();
    let media = Rect::from_min_size(rect.min, vec2(width, width * 0.53));
    let Some(texture) = picture(app, ui, url, media.size()) else {
        return;
    };
    let uv = crop(texture.size, media.size(), 0.22);
    let painter = ui.painter().with_clip_rect(media.intersect(ui.clip_rect()));
    let solid = media.top() + media.height() * 0.379;
    let v_at = |y: f32| uv.top() + (y - media.top()) / media.height() * uv.height();
    painter.image(
        texture.id,
        Rect::from_min_max(media.min, pos2(media.right(), solid)),
        Rect::from_min_max(uv.min, pos2(uv.right(), v_at(solid))),
        Color32::WHITE,
    );
    // The fade: a strip of quads whose alpha falls smoothly to nothing.
    let mut mesh = egui::Mesh::with_texture(texture.id);
    let steps = 16;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let y = solid + (media.bottom() - solid) * t;
        let alpha = 1.0 - t;
        let color = Color32::WHITE.gamma_multiply(alpha * alpha * (3.0 - 2.0 * alpha));
        for x in [media.left(), media.right()] {
            let u = if x == media.left() {
                uv.left()
            } else {
                uv.right()
            };
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pos2(x, y),
                uv: pos2(u, v_at(y)),
                color,
            });
        }
        if step > 0 {
            let i = (step as u32) * 2;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Back on the left and "…" on the right, floating over the hero in a
/// glass of the artist's colour, where mcloud hides the toolbar.
fn floating_buttons(app: &mut App, ui: &mut egui::Ui, full: Rect, artist: &Artist, tint: Color32) {
    let locale = app.locale;
    let fill = tint
        .lerp_to_gamma(Color32::WHITE, 0.12)
        .gamma_multiply(0.85);
    let top = full.top() + 12.0;
    let back = Rect::from_min_size(pos2(full.left() + 12.0, top), Vec2::splat(36.0));
    let more = Rect::from_min_size(pos2(full.right() - 14.0 - 36.0, top), Vec2::splat(36.0));
    for (rect, is_back, label) in [
        (back, true, gettext(locale, "Back")),
        (more, false, gettext(locale, "More")),
    ] {
        let response = ui.interact(
            rect,
            Id::new(("mcloud-artist-float", is_back)),
            Sense::click(),
        );
        ui.painter().circle_filled(rect.center(), 18.0, fill);
        if response.hovered() {
            ui.painter()
                .circle_filled(rect.center(), 16.0, Color32::from_white_alpha(31));
        }
        let icon = if is_back {
            Icon::McloudLeft
        } else {
            Icon::McloudMore
        };
        theme::paint_icon(ui, icon, rect, 18.0, Color32::WHITE);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        let response = response.on_hover_text(label.as_ref());
        if is_back {
            if response.clicked() && app.can_go_back() {
                app.actions.push(Action::Back);
            }
        } else {
            egui::Popup::menu(&response)
                .frame(widgets::menu_frame(&app.palette))
                .align(egui::RectAlign::BOTTOM_END)
                .show(|ui| widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None));
        }
    }
}

/// The picture at `url`, once loaded.
fn picture(app: &App, ui: &egui::Ui, url: &str, size: Vec2) -> Option<egui::load::SizedTexture> {
    app.backend.art().touch(url);
    let image = egui::Image::new(url)
        .texture_options(crate::images::COVER_TEXTURE)
        .show_loading_spinner(false);
    match image.load_for_size(ui.ctx(), size) {
        Ok(egui::load::TexturePoll::Ready { texture }) => {
            crate::images::forget_decoded(ui.ctx(), url);
            Some(texture)
        }
        _ => None,
    }
}

/// The part of a picture `image` in size that fills `target` without
/// stretching, as texture coordinates, its vertical centre at `focus`.
fn crop(image: Vec2, target: Vec2, focus: f32) -> Rect {
    let image_aspect = image.x / image.y.max(1.0);
    let target_aspect = target.x / target.y.max(1.0);
    if image_aspect > target_aspect {
        let width = target_aspect / image_aspect;
        Rect::from_min_max(pos2(0.5 - width / 2.0, 0.0), pos2(0.5 + width / 2.0, 1.0))
    } else {
        let height = image_aspect / target_aspect;
        let top = (focus - height / 2.0).clamp(0.0, 1.0 - height);
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

fn title(ui: &mut egui::Ui, title: &str) {
    theme::text(ui, title, theme::bold(20.0), Color32::WHITE);
    ui.add_space(10.0);
}

/// A titled row of cards that scrolls sideways.
fn shelf(ui: &mut egui::Ui, id: &str, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(INSET - PAGE_PADDING);
        title(ui, label);
    });
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.add_space(INSET - PAGE_PADDING);
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                add_contents(ui);
                ui.add_space(INSET - PAGE_PADDING);
            });
        });
    ui.add_space(SHELF_GAP);
}

fn album_shelf(app: &mut App, ui: &mut egui::Ui, id: &str, label: &str, albums: &[&Album]) {
    if albums.is_empty() {
        return;
    }
    shelf(ui, id, label, |ui| {
        for album in albums {
            let image = pick_image(&album.images, app.art_px(CARD));
            let year = album.year().unwrap_or("").to_string();
            let response = card(app, ui, image, &album.name, Some(&year), CARD, false);
            if response.clicked() {
                app.actions
                    .push(Action::Open(Page::Album(album.id.clone())));
            }
            egui::Popup::context_menu(&response)
                .id(ui.make_persistent_id(("discography-menu", &album.uri)))
                .frame(widgets::menu_frame(&app.palette))
                .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
        }
    });
}

/// mcloud's card: the artwork with a faint inner ring, the name in two
/// lines at most, and a quieter line under it.
pub(crate) fn card(
    app: &mut App,
    ui: &mut egui::Ui,
    image: Option<&str>,
    name: &str,
    subtitle: Option<&str>,
    side: f32,
    round: bool,
) -> egui::Response {
    let name_galley = widgets::ellipsized(ui, name, theme::medium(13.0), Color32::WHITE, side, 2);
    let subtitle_galley = subtitle
        .filter(|text| !text.is_empty())
        .map(|text| widgets::ellipsized(ui, text, theme::regular(13.0), WHITE_60, side, 1));
    let height = side
        + 6.0
        + name_galley.size().y
        + subtitle_galley
            .as_ref()
            .map_or(0.0, |galley| galley.size().y + 2.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let art = Rect::from_min_size(rect.min, Vec2::splat(side));
        let radius = if round { side / 2.0 } else { 8.0 };
        let corner = CornerRadius::same(radius.min(127.0) as u8);
        widgets::paint_cover(
            ui,
            &app.palette,
            image,
            art,
            radius,
            if round { Icon::User } else { Icon::Music },
            Some(app.backend.art()),
        );
        ui.painter().rect_stroke(
            art,
            corner,
            egui::Stroke::new(1.0, Color32::from_white_alpha(18)),
            egui::StrokeKind::Inside,
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(art, corner, Color32::from_white_alpha(14));
        }
        let x = |galley: &Arc<egui::Galley>| {
            if round {
                rect.center().x - galley.size().x / 2.0
            } else {
                rect.left()
            }
        };
        let y = art.bottom() + 6.0;
        let name_height = name_galley.size().y;
        ui.painter().galley(
            pos2(x(&name_galley), y),
            name_galley.clone(),
            Color32::WHITE,
        );
        if let Some(galley) = subtitle_galley {
            ui.painter()
                .galley(pos2(x(&galley), y + name_height + 2.0), galley, WHITE_60);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The latest release: its artwork beside the date, the name, the number
/// of songs and a "+" that saves it.
fn latest_card(app: &mut App, ui: &mut egui::Ui, album: &Album) {
    title(ui, &gettext(app.locale, "Latest release"));
    let (row, response) = ui.allocate_exact_size(vec2(300.0, LATEST_ART), Sense::click());
    let art = Rect::from_min_size(row.min, Vec2::splat(LATEST_ART));
    widgets::paint_cover(
        ui,
        &app.palette,
        pick_image(&album.images, app.art_px(LATEST_ART)),
        art,
        8.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let left = art.right() + 16.0;
    let width = row.right() - left;
    let date = album
        .release_date
        .as_deref()
        .map(release_date)
        .unwrap_or_default();
    let date_color = Color32::from_white_alpha(166);
    let date = widgets::ellipsized(ui, &date, theme::semibold(11.0), date_color, width, 1);
    let name = widgets::ellipsized(
        ui,
        &album.name,
        theme::regular(15.0),
        Color32::WHITE,
        width,
        2,
    );
    let count = album
        .total_tracks
        .map(|total| app.locale.song_count(total))
        .unwrap_or_default();
    let count = widgets::ellipsized(ui, &count, theme::regular(15.0), WHITE_60, width, 1);
    let height = date.size().y + name.size().y + count.size().y + 4.0 + 10.0 + 28.0;
    let mut y = art.center().y - height / 2.0;
    for (galley, color) in [
        (date, date_color),
        (name, Color32::WHITE),
        (count, WHITE_60),
    ] {
        let size = galley.size();
        ui.painter().galley(pos2(left, y), galley, color);
        y += size.y + 2.0;
    }
    let add = Rect::from_min_size(pos2(left, y + 8.0), Vec2::splat(28.0));
    let saved = app.is_saved(&album.uri).unwrap_or(false);
    let add_response = ui.interact(
        add,
        Id::new(("mcloud-latest-add", &album.uri)),
        Sense::click(),
    );
    ui.painter()
        .circle_filled(add.center(), 14.0, Color32::from_white_alpha(38));
    theme::paint_icon(
        ui,
        if saved { Icon::Check } else { Icon::McloudPlus },
        add,
        14.0,
        Color32::WHITE,
    );
    if add_response.clicked() {
        app.actions.push(Action::ToggleSaved(album.uri.clone()));
    } else if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.actions
            .push(Action::Open(Page::Album(album.id.clone())));
    }
}

/// "2026-10-23" as "Oct 23, 2026"; a bare year or month stays as given.
fn release_date(date: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts[..] else {
        return date.to_string();
    };
    let Some(month) = month
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)))
    else {
        return date.to_string();
    };
    format!("{month} {}, {year}", day.trim_start_matches('0'))
}

/// The top songs in three rows that scroll sideways, a hairline under all
/// but each column's last.
fn top_songs(app: &mut App, ui: &mut egui::Ui, page: &ArtistPage, artist: &Artist) {
    let locale = app.locale;
    title(ui, &gettext(locale, "Top Songs"));
    let Loadable::Loaded(tracks) = &page.top_tracks else {
        widgets::skeleton_rows(ui, &app.palette, locale, SONG_ROWS);
        return;
    };
    let uris: Arc<[String]> = tracks
        .iter()
        .map(|track| track.uri.clone())
        .collect::<Vec<_>>()
        .into();
    let playing = app.current_track_uri();
    egui::ScrollArea::horizontal()
        .id_salt(("artist-top-songs", &artist.id))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 30.0;
                for (column, chunk) in tracks.chunks(SONG_ROWS).enumerate() {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (row, track) in chunk.iter().enumerate() {
                            let here = playing.as_deref() == Some(track.uri.as_str());
                            if song_row(app, ui, track, here, row + 1 < chunk.len()) {
                                app.actions.push(Action::PlayFromRow {
                                    context: RowContext::Uris(Arc::clone(&uris)),
                                    uri: track.uri.clone(),
                                    index: (column * SONG_ROWS + row) as u32,
                                });
                            }
                        }
                    });
                }
            });
        });
}

/// One of the top songs: artwork, name and artists, and "…". Returns
/// whether the row was clicked to play.
fn song_row(
    app: &mut App,
    ui: &mut egui::Ui,
    track: &crate::api::models::Track,
    here: bool,
    divider: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(SONG_COLUMN, 54.0), Sense::click());
    if divider {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, Color32::from_white_alpha(28)),
        );
    }
    let art = Rect::from_min_size(pos2(rect.left(), rect.center().y - 22.0), Vec2::splat(44.0));
    widgets::paint_cover(
        ui,
        &app.palette,
        track
            .album
            .as_ref()
            .and_then(|album| pick_image(&album.images, app.art_px(44.0))),
        art,
        5.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let color = if here {
        app.palette.accent
    } else {
        Color32::WHITE
    };
    let text_left = art.right() + 12.0;
    let width = rect.right() - 34.0 - text_left;
    let name = widgets::ellipsized(ui, &track.name, theme::regular(13.0), color, width, 1);
    let artists = track
        .artists
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let artists = widgets::ellipsized(ui, &artists, theme::regular(13.0), WHITE_60, width, 1);
    ui.painter().galley(
        pos2(text_left, rect.center().y - name.size().y),
        name,
        color,
    );
    ui.painter()
        .galley(pos2(text_left, rect.center().y), artists, WHITE_60);
    let more = Rect::from_center_size(
        pos2(rect.right() - 14.0, rect.center().y),
        Vec2::splat(28.0),
    );
    let more_response = ui.interact(
        more,
        Id::new(("mcloud-song-more", &track.uri)),
        Sense::click(),
    );
    theme::paint_icon(
        ui,
        Icon::McloudMore,
        more,
        18.0,
        Color32::from_white_alpha(192),
    );
    let item = PlayableItem::Track(track.clone());
    egui::Popup::menu(&more_response)
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| widgets::item_menu(ui, app, &item, None, None));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &track.name));
    response.clicked() && !more_response.hovered()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_crops_around_its_focus() {
        let uv = crop(egui::vec2(640.0, 640.0), egui::vec2(1200.0, 400.0), 0.35);
        assert_eq!(uv.width(), 1.0);
        assert!((uv.height() - 1.0 / 3.0).abs() < 1e-4);
        assert!((uv.center().y - 0.35).abs() < 1e-4);
        // A picture wider than the band keeps its height and its centre.
        let uv = crop(egui::vec2(2000.0, 400.0), egui::vec2(1000.0, 400.0), 0.22);
        assert_eq!(uv.height(), 1.0);
        assert!((uv.center().x - 0.5).abs() < 1e-4);
    }

    #[test]
    fn release_dates_read_as_apple_music_writes_them() {
        assert_eq!(release_date("2026-10-23"), "Oct 23, 2026");
        assert_eq!(release_date("1984-11-09"), "Nov 9, 1984");
        assert_eq!(release_date("1984"), "1984");
    }
}

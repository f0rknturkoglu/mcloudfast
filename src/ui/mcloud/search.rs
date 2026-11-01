//! Search results as Spotify draws them under mcloud, which leaves its
//! search page alone: a column at most 1104pt wide, centred; the top result
//! as a band across it, with Follow and Play; then songs, playlists and
//! albums in one list, each row named by its kind.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{PlayableItem, SearchResults, join_names, pick_image};
use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Page};
use crate::theme::{self, Icon};
use crate::ui::widgets;

/// Spotify's search column.
pub(in crate::ui) const MAX_WIDTH: f32 = 1104.0;
const BAND_HEIGHT: f32 = 104.0;
const BAND_ART: f32 = 72.0;
const ROW_HEIGHT: f32 = 80.0;
const ROW_ART: f32 = 64.0;
/// Rows in the mixed list.
const ROWS: usize = 10;

/// Lays `add` out in the centred column.
pub(in crate::ui) fn column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().min(MAX_WIDTH);
    let inset = ((ui.available_width() - width) / 2.0).max(0.0);
    let rect = Rect::from_min_size(ui.cursor().min + vec2(inset, 0.0), vec2(width, 0.0));
    let response = ui.scope_builder(
        egui::UiBuilder::new().max_rect(rect.with_max_y(f32::INFINITY)),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
    let height = response.response.rect.height();
    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
}

/// One row of the mixed list.
enum Row<'a> {
    Song(&'a crate::api::models::Track),
    Playlist(&'a crate::api::models::Playlist),
    Album(&'a crate::api::models::Album),
}

/// The band and the mixed list.
pub(in crate::ui) fn top(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    band(app, ui, results);
    ui.add_space(24.0);
    let songs = results
        .tracks
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    let playlists = results
        .playlists
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    let albums = results
        .albums
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    for row in mix(songs, playlists, albums) {
        list_row(app, ui, row);
    }
}

/// Songs and collections in turn, as Spotify alternates them: a song, a
/// playlist, a song, an album, until the list is full.
fn mix<'a>(
    songs: &'a [crate::api::models::Track],
    playlists: &'a [crate::api::models::Playlist],
    albums: &'a [crate::api::models::Album],
) -> Vec<Row<'a>> {
    let mut songs = songs.iter().map(Row::Song);
    let mut playlists = playlists.iter().map(Row::Playlist);
    let mut albums = albums.iter().map(Row::Album);
    let mut rows = Vec::with_capacity(ROWS);
    let mut turn = 0;
    while rows.len() < ROWS {
        let next = match turn % 4 {
            0 | 2 => songs
                .next()
                .or_else(|| playlists.next())
                .or_else(|| albums.next()),
            1 => playlists
                .next()
                .or_else(|| albums.next())
                .or_else(|| songs.next()),
            _ => albums
                .next()
                .or_else(|| playlists.next())
                .or_else(|| songs.next()),
        };
        match next {
            Some(row) => rows.push(row),
            None => break,
        }
        turn += 1;
    }
    rows
}

/// The top result across the column: its picture, name and kind, then
/// Follow (for an artist) and Play at the right.
fn band(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let locale = app.locale;
    let palette = app.palette;
    let art_px = app.art_px(BAND_ART);
    let query = app.search.committed.to_lowercase();
    let artist = results
        .artists
        .as_ref()
        .and_then(|page| page.items.first())
        .filter(|artist| {
            artist.name.to_lowercase() == query
                || results.tracks.as_ref().is_none_or(|t| t.items.is_empty())
        });
    let (uri, name, kind, image, page, round) = if let Some(artist) = artist {
        (
            artist.uri.clone(),
            artist.name.clone(),
            gettext(locale, "Artist").into_owned(),
            pick_image(&artist.images, art_px).map(str::to_string),
            Page::Artist(artist.id.clone()),
            true,
        )
    } else if let Some(track) = results.tracks.as_ref().and_then(|page| page.items.first()) {
        (
            track.uri.clone(),
            track.name.clone(),
            join_names(track.artists.iter().map(|a| a.name.as_str())),
            track.image(art_px).map(str::to_string),
            track
                .album
                .as_ref()
                .map(|album| Page::Album(album.id.clone()))
                .unwrap_or(Page::Search),
            false,
        )
    } else if let Some(album) = results.albums.as_ref().and_then(|page| page.items.first()) {
        (
            album.uri.clone(),
            album.name.clone(),
            gettext(locale, "Album").into_owned(),
            pick_image(&album.images, art_px).map(str::to_string),
            Page::Album(album.id.clone()),
            false,
        )
    } else {
        return;
    };

    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), BAND_HEIGHT), Sense::click());
    let fill = if response.hovered() { 31 } else { 20 };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(fill));
    let art = Rect::from_min_size(
        pos2(rect.left() + 16.0, rect.center().y - BAND_ART / 2.0),
        Vec2::splat(BAND_ART),
    );
    let radius = if round { BAND_ART / 2.0 } else { 4.0 };
    let placeholder = if round { Icon::User } else { Icon::Music };
    widgets::paint_cover(
        ui,
        &palette,
        image.as_deref(),
        art,
        radius,
        placeholder,
        Some(app.backend.art()),
    );

    // Play at the right end, Follow before it.
    let play = Rect::from_center_size(
        pos2(rect.right() - 24.0 - 24.0, rect.center().y),
        Vec2::splat(48.0),
    );
    let mut right = play.left() - 16.0;
    let follow = round.then(|| {
        let following = app.is_saved(&uri).unwrap_or(false);
        let label = if following {
            pgettext(locale, "artist", "Following")
        } else {
            pgettext(locale, "artist", "Follow")
        };
        let galley = crate::bidi::layout_line(
            ui.painter(),
            label.as_ref(),
            theme::semibold(14.0),
            Color32::WHITE,
        );
        let pill = Rect::from_min_max(
            pos2(right - galley.size().x - 30.0, rect.center().y - 16.0),
            pos2(right, rect.center().y + 16.0),
        );
        right = pill.left() - 16.0;
        (pill, galley, label, following)
    });

    let text_left = art.right() + 16.0;
    let text_width = (right - text_left).max(40.0);
    let title = widgets::ellipsized(ui, &name, theme::bold(22.0), palette.text, text_width, 1);
    let sub = widgets::ellipsized(
        ui,
        &kind,
        theme::regular(14.0),
        palette.secondary,
        text_width,
        1,
    );
    let top = rect.center().y - (title.size().y + 2.0 + sub.size().y) / 2.0;
    ui.painter()
        .galley(pos2(text_left, top), title.clone(), palette.text);
    ui.painter().galley(
        pos2(text_left, top + title.size().y + 2.0),
        sub,
        palette.secondary,
    );

    if let Some((pill, galley, label, following)) = follow {
        let response = ui.interact(
            pill,
            Id::new(("mcloud-search-follow", &uri)),
            Sense::click(),
        );
        let stroke = if response.hovered() { 255 } else { 128 };
        ui.painter().rect_stroke(
            pill,
            CornerRadius::same(16),
            egui::Stroke::new(1.0, Color32::from_white_alpha(stroke)),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(pill.center() - galley.size() / 2.0, galley, Color32::WHITE);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, following, label.as_ref())
        });
        theme::focus_ring(ui, &response);
        if response.clicked() {
            app.actions.push(Action::ToggleSaved(uri.clone()));
        }
    }

    let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let play_response = ui.interact(play, Id::new(("mcloud-search-play", &uri)), Sense::click());
    let colour = if play_response.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    ui.painter().circle_filled(play.center(), 24.0, colour);
    let (icon, label) = if playing_here {
        (Icon::McloudPause, gettext(locale, "Pause"))
    } else {
        (Icon::McloudPlay, gettext(locale, "Play"))
    };
    theme::paint_icon(ui, icon, play, 22.0, Color32::WHITE);
    play_response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    theme::focus_ring(ui, &play_response);
    if play_response.on_hover_text(label.as_ref()).clicked() {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else if uri.starts_with("spotify:track:") {
            app.actions.push(Action::PlayUris {
                uris: vec![uri.clone()],
                index: 0,
            });
        } else {
            app.actions.push(Action::PlayContext {
                uri: uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    } else if response.clicked() {
        app.actions.push(Action::Open(page));
    }
    egui::Popup::context_menu(&response)
        .id(ui.make_persistent_id(("mcloud-search-band-menu", &uri)))
        .frame(widgets::menu_frame(&palette))
        .show(|ui| widgets::context_menu_items(ui, app, &uri, &name, None));
}

/// A row of the mixed list: the cover, the name over its artists or owner,
/// its kind, and + to save it.
fn list_row(app: &mut App, ui: &mut egui::Ui, row: Row<'_>) {
    let locale = app.locale;
    let palette = app.palette;
    let art_px = app.art_px(ROW_ART);
    let (uri, name, sub, image, kind) = match &row {
        Row::Song(track) => (
            track.uri.clone(),
            track.name.clone(),
            join_names(track.artists.iter().map(|a| a.name.as_str())),
            track.image(art_px).map(str::to_string),
            // Translators: The kind of a search result row.
            pgettext(locale, "search result", "Song"),
        ),
        Row::Playlist(playlist) => (
            playlist.uri.clone(),
            playlist.name.clone(),
            playlist.owner_name().to_string(),
            pick_image(&playlist.images, art_px).map(str::to_string),
            gettext(locale, "Playlist"),
        ),
        Row::Album(album) => (
            album.uri.clone(),
            album.name.clone(),
            join_names(album.artists.iter().map(|a| a.name.as_str())),
            pick_image(&album.images, art_px).map(str::to_string),
            gettext(locale, "Album"),
        ),
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), Color32::from_white_alpha(13));
    }
    let art = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.center().y - ROW_ART / 2.0),
        Vec2::splat(ROW_ART),
    );
    widgets::paint_cover(
        ui,
        &palette,
        image.as_deref(),
        art,
        4.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let kind_x = rect.left() + rect.width() * 0.74;
    let text_left = art.right() + 12.0;
    let text_width = (kind_x - text_left - 24.0).max(40.0);
    let current =
        matches!(row, Row::Song(_)) && app.now_playing().is_some_and(|now| now.uri == uri);
    let name_colour = if current {
        palette.accent
    } else {
        palette.text
    };
    let title = widgets::ellipsized(ui, &name, theme::regular(16.0), name_colour, text_width, 1);
    let subtitle = widgets::ellipsized(
        ui,
        &sub,
        theme::regular(14.0),
        palette.secondary,
        text_width,
        1,
    );
    let top = rect.center().y - (title.size().y + 2.0 + subtitle.size().y) / 2.0;
    ui.painter()
        .galley(pos2(text_left, top), title, name_colour);
    ui.painter().galley(
        pos2(text_left, top + 2.0 + 16.0 * 1.25),
        subtitle,
        palette.secondary,
    );

    // The kind, in a small grey label.
    let badge = crate::bidi::layout_line(
        ui.painter(),
        kind.as_ref(),
        theme::semibold(11.0),
        palette.secondary,
    );
    let badge_rect = Rect::from_min_size(
        pos2(kind_x, rect.center().y - 10.0),
        vec2(badge.size().x + 12.0, 20.0),
    );
    ui.painter().rect_filled(
        badge_rect,
        CornerRadius::same(4),
        Color32::from_white_alpha(20),
    );
    ui.painter().galley(
        badge_rect.center() - badge.size() / 2.0,
        badge,
        palette.secondary,
    );

    // + saves it; a saved one shows a tick in the accent.
    let saved = app.is_saved(&uri).unwrap_or(false);
    let plus = Rect::from_center_size(
        pos2(rect.right() - 24.0, rect.center().y),
        Vec2::splat(28.0),
    );
    let plus_response = ui.interact(plus, Id::new(("mcloud-search-save", &uri)), Sense::click());
    let (icon, colour, label) = if saved {
        (
            Icon::CircleCheck,
            palette.accent,
            gettext(locale, "Remove from Your Library"),
        )
    } else {
        let colour = if plus_response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        (
            Icon::CirclePlus,
            colour,
            gettext(locale, "Save to Your Library"),
        )
    };
    theme::paint_icon(ui, icon, plus, 18.0, colour);
    plus_response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    theme::focus_ring(ui, &plus_response);
    if plus_response.on_hover_text(label.as_ref()).clicked() {
        app.actions.push(Action::ToggleSaved(uri.clone()));
    } else if response.clicked() {
        match &row {
            // A search result plays on its own; Spotify continues after it.
            Row::Song(_) => app.actions.push(Action::PlayUris {
                uris: vec![uri.clone()],
                index: 0,
            }),
            Row::Playlist(playlist) => app
                .actions
                .push(Action::Open(Page::Playlist(playlist.id.clone()))),
            Row::Album(album) => app
                .actions
                .push(Action::Open(Page::Album(album.id.clone()))),
        }
    }
    let menu = egui::Popup::context_menu(&response)
        .id(ui.make_persistent_id(("mcloud-search-row-menu", &uri)))
        .frame(widgets::menu_frame(&palette));
    match &row {
        Row::Song(track) => {
            let item = PlayableItem::Track((*track).clone());
            menu.show(|ui| widgets::item_menu(ui, app, &item, None, None));
        }
        _ => {
            menu.show(|ui| widgets::context_menu_items(ui, app, &uri, &name, None));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{Album, Playlist, Track};

    /// Songs and collections alternate, songs leading, and a kind that runs
    /// out gives its turn to the others.
    #[test]
    fn the_list_alternates_songs_and_collections() {
        let songs = vec![Track::default(), Track::default(), Track::default()];
        let playlists = vec![Playlist::default()];
        let albums = vec![Album::default(), Album::default()];
        let kinds: Vec<char> = mix(&songs, &playlists, &albums)
            .iter()
            .map(|row| match row {
                Row::Song(_) => 's',
                Row::Playlist(_) => 'p',
                Row::Album(_) => 'a',
            })
            .collect();
        assert_eq!(kinds.iter().collect::<String>(), "spsasa");
    }
}

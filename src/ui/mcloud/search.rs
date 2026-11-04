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

    // The kind, in Spotify's small grey label: 16pt tall, 4pt either side.
    let badge = crate::bidi::layout_line(
        ui.painter(),
        kind.as_ref(),
        theme::bold(12.5),
        palette.secondary,
    );
    let badge_rect = Rect::from_min_size(
        pos2(kind_x, rect.center().y - 8.0),
        vec2(badge.size().x + 8.0, 16.0),
    );
    ui.painter().rect_filled(
        badge_rect,
        CornerRadius::same(3),
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

pub(crate) const POPUP_INSET_X: f32 = 1.5;
pub(crate) const POPUP_TOP_GAP: f32 = 2.5;
pub(crate) const POPUP_RADIUS: u8 = 14;
pub(crate) const POPUP_PAD_Y: f32 = 5.0;
pub(crate) const POPUP_MAX_HEIGHT: f32 = 523.0;
pub(crate) const QUERY_ROW_HEIGHT: f32 = 24.0;
pub(crate) const RESULT_ROW_HEIGHT: f32 = 52.0;
pub(crate) const SEPARATOR_HEIGHT: f32 = 11.0;

#[derive(Clone)]
pub(crate) struct SuggestionResult {
    pub uri: String,
    pub title: String,
    pub subtitle: String,
    pub image: Option<String>,
    pub round: bool,
    pub target: SuggestionTarget,
}

#[derive(Clone)]
pub(crate) enum SuggestionTarget {
    Open(Page),
    PlayTrack(String),
}

/// Computes up to 3 query completions for `query` from recent searches and
/// loaded results.
pub(crate) fn collect_query_suggestions(
    query: &str,
    history: &[String],
    results: Option<&SearchResults>,
) -> Vec<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let lower = trimmed.to_lowercase();
    let mut out: Vec<String> = Vec::with_capacity(3);
    let mut push = |candidate: &str| {
        let c = candidate.trim();
        if c.is_empty() || out.len() >= 3 {
            return;
        }
        let cl = c.to_lowercase();
        if cl.starts_with(&lower) && !out.iter().any(|existing| existing.eq_ignore_ascii_case(c)) {
            out.push(c.to_string());
        }
    };
    push(trimmed);
    for item in history {
        push(item);
    }
    if let Some(res) = results {
        if let Some(artists) = &res.artists {
            for a in &artists.items {
                push(&a.name);
            }
        }
        if let Some(tracks) = &res.tracks {
            for t in &tracks.items {
                push(&t.name);
            }
        }
        if let Some(albums) = &res.albums {
            for al in &albums.items {
                push(&al.name);
            }
        }
    }
    out.truncate(3);
    out
}

/// Splits `suggestion` into `(typed_prefix, completion_suffix)` matching `query`.
pub(crate) fn split_query_completion<'a>(query: &str, suggestion: &'a str) -> (&'a str, &'a str) {
    let trimmed = query.trim();
    let char_count = trimmed.chars().count();
    let split_byte = suggestion
        .char_indices()
        .nth(char_count)
        .map(|(idx, _)| idx)
        .unwrap_or(suggestion.len());
    let prefix = &suggestion[..split_byte];
    if prefix.eq_ignore_ascii_case(trimmed) {
        (prefix, &suggestion[split_byte..])
    } else {
        (suggestion, "")
    }
}

fn collect_result_suggestions(
    locale: crate::i18n::Locale,
    art_px: u32,
    results: &SearchResults,
) -> Vec<SuggestionResult> {
    let mut out = Vec::with_capacity(7);
    if let Some(artist) = results.artists.as_ref().and_then(|p| p.items.first()) {
        out.push(SuggestionResult {
            uri: artist.uri.clone(),
            title: artist.name.clone(),
            subtitle: gettext(locale, "Artist").into_owned(),
            image: pick_image(&artist.images, art_px).map(str::to_string),
            round: true,
            target: SuggestionTarget::Open(Page::Artist(artist.id.clone())),
        });
    }
    let songs = results
        .tracks
        .as_ref()
        .map(|p| p.items.as_slice())
        .unwrap_or_default();
    let albums = results
        .albums
        .as_ref()
        .map(|p| p.items.as_slice())
        .unwrap_or_default();
    let playlists = results
        .playlists
        .as_ref()
        .map(|p| p.items.as_slice())
        .unwrap_or_default();
    let song_kind = pgettext(locale, "search result", "Song");
    let album_kind = gettext(locale, "Album");
    let playlist_kind = gettext(locale, "Playlist");
    for row in mix(songs, playlists, albums) {
        if out.len() >= 7 {
            break;
        }
        match row {
            Row::Song(track) => {
                let artists = join_names(track.artists.iter().map(|a| a.name.as_str()));
                out.push(SuggestionResult {
                    uri: track.uri.clone(),
                    title: track.name.clone(),
                    subtitle: format!("{song_kind} · {artists}"),
                    image: track.image(art_px).map(str::to_string),
                    round: false,
                    target: SuggestionTarget::PlayTrack(track.uri.clone()),
                });
            }
            Row::Album(album) => {
                let artists = join_names(album.artists.iter().map(|a| a.name.as_str()));
                out.push(SuggestionResult {
                    uri: album.uri.clone(),
                    title: album.name.clone(),
                    subtitle: format!("{album_kind} · {artists}"),
                    image: pick_image(&album.images, art_px).map(str::to_string),
                    round: false,
                    target: SuggestionTarget::Open(Page::Album(album.id.clone())),
                });
            }
            Row::Playlist(playlist) => {
                let owner = playlist.owner_name();
                out.push(SuggestionResult {
                    uri: playlist.uri.clone(),
                    title: playlist.name.clone(),
                    subtitle: format!("{playlist_kind} · {owner}"),
                    image: pick_image(&playlist.images, art_px).map(str::to_string),
                    round: false,
                    target: SuggestionTarget::Open(Page::Playlist(playlist.id.clone())),
                });
            }
        }
    }
    out
}

/// Draws the macOS 27 Music search suggestions popover below `field_rect` when
/// the toolbar search box is focused and non-empty.
pub(crate) fn suggestions_popup(app: &mut App, ctx: &egui::Context, field_rect: Rect) {
    let search_id = Id::new("global-search");
    let focused = ctx.memory(|m| m.has_focus(search_id));
    let query = app.search.query.trim().to_string();
    let dismiss_id = Id::new("mcloud-search-popup-dismissed");
    if query.is_empty() {
        ctx.data_mut(|d| d.remove::<String>(dismiss_id));
        return;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.data_mut(|d| d.insert_temp(dismiss_id, query.clone()));
        return;
    }
    let dismissed_for = ctx.data(|d| d.get_temp::<String>(dismiss_id));
    if dismissed_for.as_deref() == Some(query.as_str()) {
        return;
    }
    let pointer_in_popup_id = Id::new("mcloud-search-popup-hovered");
    let was_hovered = ctx
        .data(|d| d.get_temp::<bool>(pointer_in_popup_id))
        .unwrap_or(false);
    if !focused && !was_hovered {
        return;
    }

    let loaded_results = app.search.results.get().cloned();
    let loading = app.search.catalogue_pending && loaded_results.is_none();
    let queries = collect_query_suggestions(
        &query,
        &app.settings.search_history,
        loaded_results.as_ref(),
    );
    let art_px = app.art_px(46.0);
    let results = loaded_results
        .as_ref()
        .map(|r| collect_result_suggestions(app.locale, art_px, r))
        .unwrap_or_default();

    if !loading && queries.is_empty() && results.is_empty() {
        return;
    }

    let q_count = if loading { 3 } else { queries.len() };
    let r_count = if loading { 3 } else { results.len() };
    let sep_h = if q_count > 0 && r_count > 0 {
        SEPARATOR_HEIGHT
    } else {
        0.0
    };
    let content_h = POPUP_PAD_Y * 2.0
        + q_count as f32 * QUERY_ROW_HEIGHT
        + sep_h
        + r_count as f32 * RESULT_ROW_HEIGHT;
    let popup_h = content_h.min(POPUP_MAX_HEIGHT);
    let popup_rect = Rect::from_min_max(
        pos2(
            field_rect.left() + POPUP_INSET_X,
            field_rect.bottom() + POPUP_TOP_GAP,
        ),
        pos2(
            (field_rect.right() - POPUP_INSET_X).max(field_rect.left() + 180.0),
            field_rect.bottom() + POPUP_TOP_GAP + popup_h,
        ),
    );

    let area_resp = egui::Area::new(Id::new("mcloud-search-suggestions-popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(popup_rect.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(popup_rect.size(), Sense::hover());
            let glass = super::glass::paint(ui, rect, app.settings.mcloud_glass);
            // Glass brightness 44 + 0.044*L, saturation 1.14x
            let fill = Color32::from_rgba_unmultiplied(46, 46, 49, if glass { 198 } else { 242 });
            ui.painter()
                .rect_filled(rect, CornerRadius::same(POPUP_RADIUS), fill);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(POPUP_RADIUS),
                egui::Stroke::new(0.5, Color32::from_white_alpha(38)),
                egui::StrokeKind::Inside,
            );

            let mut y = rect.top() + POPUP_PAD_Y;
            if loading {
                // 3 query skeletons + separator + 3 result skeletons
                for _ in 0..3 {
                    let row_rect = Rect::from_min_size(
                        pos2(rect.left(), y),
                        vec2(rect.width(), QUERY_ROW_HEIGHT),
                    );
                    let icon_rect = Rect::from_center_size(
                        pos2(row_rect.left() + 17.5 + 6.25, row_rect.center().y),
                        Vec2::splat(12.5),
                    );
                    theme::paint_icon(
                        ui,
                        Icon::Search,
                        icon_rect,
                        12.5,
                        Color32::from_white_alpha(90),
                    );
                    let bar = Rect::from_min_size(
                        pos2(row_rect.left() + 38.5, row_rect.center().y - 5.0),
                        vec2((rect.width() * 0.42).clamp(80.0, 180.0), 10.0),
                    );
                    ui.painter().rect_filled(
                        bar,
                        CornerRadius::same(4),
                        Color32::from_white_alpha(28),
                    );
                    y += QUERY_ROW_HEIGHT;
                }
                let sep_y = y + SEPARATOR_HEIGHT / 2.0;
                ui.painter().hline(
                    (rect.left() + 16.0)..=(rect.right() - 16.0),
                    sep_y,
                    egui::Stroke::new(1.0, Color32::from_white_alpha(77)),
                );
                y += SEPARATOR_HEIGHT;
                for _ in 0..3 {
                    let row_rect = Rect::from_min_size(
                        pos2(rect.left(), y),
                        vec2(rect.width(), RESULT_ROW_HEIGHT),
                    );
                    let cover = Rect::from_min_size(
                        pos2(row_rect.left() + 17.0, row_rect.center().y - 23.0),
                        Vec2::splat(46.0),
                    );
                    ui.painter().rect_filled(
                        cover,
                        CornerRadius::same(5),
                        Color32::from_white_alpha(28),
                    );
                    let title_bar = Rect::from_min_size(
                        pos2(cover.right() + 10.0, row_rect.center().y - 12.0),
                        vec2((rect.width() * 0.48).clamp(90.0, 200.0), 10.0),
                    );
                    let sub_bar = Rect::from_min_size(
                        pos2(cover.right() + 10.0, row_rect.center().y + 3.0),
                        vec2((rect.width() * 0.32).clamp(60.0, 140.0), 8.0),
                    );
                    ui.painter().rect_filled(
                        title_bar,
                        CornerRadius::same(4),
                        Color32::from_white_alpha(28),
                    );
                    ui.painter().rect_filled(
                        sub_bar,
                        CornerRadius::same(4),
                        Color32::from_white_alpha(18),
                    );
                    y += RESULT_ROW_HEIGHT;
                }
                return;
            }

            let palette = app.palette;
            let mut chosen_query = None;
            for (idx, q) in queries.iter().enumerate() {
                let row_rect =
                    Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), QUERY_ROW_HEIGHT));
                let resp = ui.interact(
                    row_rect,
                    Id::new(("mcloud-search-sug-q", idx)),
                    Sense::click(),
                );
                let selected = resp.hovered();
                if selected {
                    let sel_rect = Rect::from_min_max(
                        pos2(rect.left() + 5.0, row_rect.top()),
                        pos2(rect.right() - 5.0, row_rect.bottom()),
                    );
                    ui.painter()
                        .rect_filled(sel_rect, CornerRadius::same(6), palette.accent);
                }
                let icon_rect = Rect::from_center_size(
                    pos2(row_rect.left() + 17.5 + 6.25, row_rect.center().y),
                    Vec2::splat(12.5),
                );
                theme::paint_icon(
                    ui,
                    Icon::Search,
                    icon_rect,
                    12.5,
                    if selected {
                        Color32::WHITE
                    } else {
                        Color32::from_white_alpha(160)
                    },
                );
                let (prefix, suffix) = split_query_completion(&query, q);
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    prefix,
                    0.0,
                    egui::TextFormat {
                        font_id: theme::regular(13.0),
                        color: if selected {
                            Color32::WHITE
                        } else {
                            Color32::from_white_alpha(217)
                        },
                        ..Default::default()
                    },
                );
                if !suffix.is_empty() {
                    job.append(
                        suffix,
                        0.0,
                        egui::TextFormat {
                            font_id: theme::regular(13.0),
                            color: if selected {
                                Color32::WHITE
                            } else {
                                Color32::from_white_alpha(140)
                            },
                            ..Default::default()
                        },
                    );
                }
                let galley = ui.painter().layout_job(job);
                let text_pos = pos2(
                    row_rect.left() + 38.5,
                    row_rect.center().y - galley.size().y / 2.0,
                );
                ui.painter().galley(text_pos, galley, Color32::WHITE);
                if resp.clicked() {
                    chosen_query = Some(q.clone());
                }
                y += QUERY_ROW_HEIGHT;
            }

            if !queries.is_empty() && !results.is_empty() {
                let sep_y = y + SEPARATOR_HEIGHT / 2.0;
                ui.painter().hline(
                    (rect.left() + 16.0)..=(rect.right() - 16.0),
                    sep_y,
                    egui::Stroke::new(1.0, Color32::from_white_alpha(77)),
                );
                y += SEPARATOR_HEIGHT;
            }

            let mut chosen_target = None;
            for (idx, item) in results.iter().enumerate() {
                let row_rect = Rect::from_min_size(
                    pos2(rect.left(), y),
                    vec2(rect.width(), RESULT_ROW_HEIGHT),
                );
                let resp = ui.interact(
                    row_rect,
                    Id::new(("mcloud-search-sug-r", idx, &item.uri)),
                    Sense::click(),
                );
                let selected = resp.hovered();
                if selected {
                    let sel_rect = Rect::from_min_max(
                        pos2(rect.left() + 5.0, row_rect.top()),
                        pos2(rect.right() - 5.0, row_rect.bottom()),
                    );
                    ui.painter()
                        .rect_filled(sel_rect, CornerRadius::same(6), palette.accent);
                }
                let cover_rect = Rect::from_min_size(
                    pos2(row_rect.left() + 17.0, row_rect.center().y - 23.0),
                    Vec2::splat(46.0),
                );
                let cover_radius = if item.round { 23.0 } else { 5.0 };
                let placeholder = if item.round { Icon::User } else { Icon::Music };
                widgets::paint_cover(
                    ui,
                    &palette,
                    item.image.as_deref(),
                    cover_rect,
                    cover_radius,
                    placeholder,
                    Some(app.backend.art()),
                );
                let text_left = cover_rect.right() + 10.0;
                let text_w = (rect.right() - 14.0 - text_left).max(40.0);
                let title_col = if selected {
                    Color32::WHITE
                } else {
                    palette.text
                };
                let sub_col = if selected {
                    Color32::from_white_alpha(230)
                } else {
                    palette.secondary
                };
                let title_galley =
                    widgets::ellipsized(ui, &item.title, theme::medium(13.0), title_col, text_w, 1);
                let sub_galley = widgets::ellipsized(
                    ui,
                    &item.subtitle,
                    theme::regular(11.0),
                    sub_col,
                    text_w,
                    1,
                );
                let block_top =
                    row_rect.center().y - (title_galley.size().y + 2.0 + sub_galley.size().y) / 2.0;
                let title_h = title_galley.size().y;
                ui.painter()
                    .galley(pos2(text_left, block_top), title_galley, title_col);
                ui.painter().galley(
                    pos2(text_left, block_top + title_h + 2.0),
                    sub_galley,
                    sub_col,
                );
                if resp.clicked() {
                    chosen_target = Some(item.target.clone());
                }
                y += RESULT_ROW_HEIGHT;
            }

            if let Some(q) = chosen_query {
                app.search.query = q.clone();
                app.actions.push(Action::Search(q.clone()));
                app.actions.push(Action::Open(Page::Search));
                ctx.data_mut(|d| d.insert_temp(dismiss_id, q));
            } else if let Some(target) = chosen_target {
                match target {
                    SuggestionTarget::Open(page) => app.actions.push(Action::Open(page)),
                    SuggestionTarget::PlayTrack(uri) => app.actions.push(Action::PlayUris {
                        uris: vec![uri],
                        index: 0,
                    }),
                }
                ctx.data_mut(|d| d.insert_temp(dismiss_id, query.clone()));
            }
        });

    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
    let hovered_now = pointer_pos.is_some_and(|p| area_resp.response.rect.contains(p));
    ctx.data_mut(|d| d.insert_temp(pointer_in_popup_id, hovered_now));
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

    #[test]
    fn query_suggestions_and_completion_split_match_prefix() {
        let history = vec!["daft punk".into(), "radiohead".into(), "daft".into()];
        let sug = collect_query_suggestions("daf", &history, None);
        assert_eq!(sug, vec!["daf", "daft punk", "daft"]);
        assert_eq!(
            split_query_completion("daf", "daft punk"),
            ("daf", "t punk")
        );
    }
}

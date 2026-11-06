//! Search results as mcloud draws them, in Apple Music's sections: Top Results
//! as plates, then a shelf for each kind, whose heading opens that kind's own
//! page (Apple's "see all"): Songs as a table, the rest as a grid.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{
    Album, Artist, PlayableItem, Playlist, SearchResults, Track, join_names, pick_image,
};
use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Page, SearchFilter};
use crate::theme::{self, Icon};
use crate::ui::{search, widgets};

/// The page's margin (user.css `--am-sp`).
const SIDE: f32 = 34.0;
const TOP_HEIGHT: f32 = 80.0;
const TOP_ART: f32 = 42.0;
const SONG_ROW: f32 = 56.0;
const SONG_ART: f32 = 48.0;
const TABLE_ART: f32 = 40.0;
const TABLE_HEAD: f32 = 30.0;
const SONGS_PER_COLUMN: usize = 3;

/// How many of each kind a row shows, and the gap between them, by the width
/// beside the sidebar (user.css's container queries on `#am-search`, measured
/// from Music). A shelf's step is its width over the count, so the next card
/// starts at the right margin.
#[derive(Debug, PartialEq)]
struct Shelves {
    gap: f32,
    top: usize,
    album: usize,
    artist: usize,
    song: usize,
}

fn shelves(width: f32) -> Shelves {
    let (gap, top, mut album, mut artist, mut song) = if width >= 1060.0 {
        (20.0, 4, 5, 6, 3)
    } else if width >= 640.0 {
        (16.0, 3, 4, 5, 2)
    } else {
        (16.0, 2, 3, 4, 1)
    };
    if width >= 1250.0 {
        (album, artist) = (6, 7);
    }
    if width >= 1450.0 {
        (album, artist, song) = (7, 8, 4);
    }
    Shelves {
        gap,
        top,
        album,
        artist,
        song,
    }
}

/// Draws `add` across the whole width beside the sidebar, past the page's own
/// margins: the sections keep mcloud's, and their rows run to the edges.
fn full_width(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let side = super::page_padding(ui.ctx());
    let room = ui.available_rect_before_wrap();
    let mut page = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
        pos2(room.left() - side, room.top()),
        pos2(room.right() + side, room.bottom()),
    )));
    add(&mut page);
    let bottom = page.min_rect().bottom();
    ui.advance_cursor_after_rect(Rect::from_min_max(room.min, pos2(room.right(), bottom)));
}

/// The results under mcloud: every kind's section, or one kind's page.
pub(in crate::ui) fn results(
    app: &mut App,
    ui: &mut egui::Ui,
    results: &SearchResults,
    kind: SearchFilter,
) {
    full_width(ui, |ui| match kind {
        SearchFilter::All => sections(app, ui, results),
        kind => kind_page(app, ui, results, kind),
    });
}

/// A section's heading, which opens the kind's page when it has one.
fn heading(app: &mut App, ui: &mut egui::Ui, label: &str, kind: Option<SearchFilter>) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.add_space(SIDE);
        match kind {
            Some(kind) => {
                if theme::section_link(ui, &palette, label).clicked() {
                    app.actions.push(Action::Open(Page::SearchKind(kind)));
                }
            }
            None => {
                theme::section_title(ui, &palette, label);
            }
        }
    });
    ui.add_space(13.0 - ui.spacing().item_spacing.y);
}

fn sections(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let locale = app.locale;
    let width = ui.available_width();
    let counts = shelves(width);
    let inner = width - 2.0 * SIDE;
    ui.add_space(14.0);
    top_results(app, ui, results, &counts);

    let art = |count: usize| inner / count as f32 - counts.gap;
    let artists = results
        .artists
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    if !artists.is_empty() {
        let size = art(counts.artist);
        shelf(
            app,
            ui,
            "search-artists",
            &gettext(locale, "Artists"),
            SearchFilter::Artists,
            size + counts.gap,
            counts.gap,
            |app, ui| {
                for artist in artists {
                    search::artist_card(app, ui, artist, Some(size));
                }
            },
        );
    }
    let albums = results
        .albums
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    if !albums.is_empty() {
        let size = art(counts.album);
        shelf(
            app,
            ui,
            "search-albums",
            &gettext(locale, "Albums"),
            SearchFilter::Albums,
            size + counts.gap,
            counts.gap,
            |app, ui| {
                for album in albums {
                    search::album_card(app, ui, album, Some(size));
                }
            },
        );
    }
    let tracks = results
        .tracks
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    if !tracks.is_empty() {
        let column = inner / counts.song as f32 - counts.gap;
        shelf(
            app,
            ui,
            "search-songs",
            &gettext(locale, "Songs"),
            SearchFilter::Songs,
            column + counts.gap,
            counts.gap,
            |app, ui| {
                for songs in tracks.chunks(SONGS_PER_COLUMN) {
                    let (block, _) = ui.allocate_exact_size(
                        vec2(column, SONG_ROW * SONGS_PER_COLUMN as f32),
                        Sense::hover(),
                    );
                    for (index, track) in songs.iter().enumerate() {
                        let rect = Rect::from_min_size(
                            block.min + vec2(0.0, SONG_ROW * index as f32),
                            vec2(column, SONG_ROW),
                        );
                        song_row(app, ui, rect, track, index + 1 < SONGS_PER_COLUMN);
                    }
                }
            },
        );
    }
    let playlists = results
        .playlists
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    if !playlists.is_empty() {
        let size = art(counts.album);
        shelf(
            app,
            ui,
            "search-playlists",
            &gettext(locale, "Playlists"),
            SearchFilter::Playlists,
            size + counts.gap,
            counts.gap,
            |app, ui| {
                for playlist in playlists {
                    search::playlist_card(app, ui, playlist, Some(size));
                }
            },
        );
    }
    let shows = results
        .shows
        .as_ref()
        .map(|page| page.items.as_slice())
        .unwrap_or_default();
    if !shows.is_empty() {
        let size = art(counts.album);
        shelf(
            app,
            ui,
            "search-shows",
            &gettext(locale, "Podcasts & Shows"),
            SearchFilter::Podcasts,
            size + counts.gap,
            counts.gap,
            |app, ui| {
                for show in shows {
                    search::show_card(app, ui, show, Some(size));
                }
            },
        );
    }
}

#[expect(clippy::too_many_arguments)]
fn shelf(
    app: &mut App,
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    kind: SearchFilter,
    pitch: f32,
    gap: f32,
    add: impl FnOnce(&mut App, &mut egui::Ui),
) {
    ui.add_space(25.0);
    heading(app, ui, title, Some(kind));
    shelf_row(ui, id, pitch, gap, |ui| add(app, ui));
}

/// A shelf's row: the first card on the page's margin, and room for the last
/// to scroll clear of the other. Space is added after an item, not before it.
fn shelf_row(ui: &mut egui::Ui, id: &str, pitch: f32, gap: f32, add: impl FnOnce(&mut egui::Ui)) {
    widgets::snapping_row(ui, id, pitch, |ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.add_space(SIDE);
        add(ui);
        ui.add_space(SIDE - gap);
    });
}

/// What Top Results lists: Spotify's own relevance order is not in the Web
/// API's answer, so the best artist leads and the kinds take turns after it.
fn top_items<'a>(results: &'a SearchResults, count: usize) -> Vec<Row<'a>> {
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
    let artist = results.artists.as_ref().and_then(|page| page.items.first());
    artist
        .map(Row::Artist)
        .into_iter()
        .chain(mix(songs, playlists, albums))
        .take(count)
        .collect()
}

/// Top Results: two rows of plates, with what a third would hold left out.
fn top_results(app: &mut App, ui: &mut egui::Ui, results: &SearchResults, counts: &Shelves) {
    let items = top_items(results, counts.top * 2);
    if items.is_empty() {
        return;
    }
    heading(app, ui, &gettext(app.locale, "Top Results"), None);
    let inner = ui.available_width() - 2.0 * SIDE;
    let plate = (inner - counts.gap * (counts.top as f32 - 1.0)) / counts.top as f32;
    let rows = items.len().div_ceil(counts.top);
    let (block, _) = ui.allocate_exact_size(
        vec2(
            ui.available_width(),
            TOP_HEIGHT * rows as f32 + counts.gap * (rows as f32 - 1.0),
        ),
        Sense::hover(),
    );
    for (index, item) in items.iter().enumerate() {
        let (row, column) = (index / counts.top, index % counts.top);
        let rect = Rect::from_min_size(
            block.min
                + vec2(
                    SIDE + (plate + counts.gap) * column as f32,
                    (TOP_HEIGHT + counts.gap) * row as f32,
                ),
            vec2(plate, TOP_HEIGHT),
        );
        top_plate(app, ui, rect, item);
    }
}

/// One entry of Top Results: a song plays and has "…", the rest open and end
/// in a chevron.
fn top_plate(app: &mut App, ui: &mut egui::Ui, rect: Rect, item: &Row<'_>) {
    let locale = app.locale;
    let palette = app.palette;
    let art_px = app.art_px(TOP_ART);
    let (uri, name, kind, by, image, round, explicit) = match item {
        Row::Artist(artist) => (
            artist.uri.clone(),
            artist.name.clone(),
            gettext(locale, "Artist"),
            String::new(),
            pick_image(&artist.images, art_px).map(str::to_string),
            true,
            false,
        ),
        Row::Song(track) => (
            track.uri.clone(),
            track.name.clone(),
            // Translators: The kind of a search result row.
            pgettext(locale, "search result", "Song"),
            join_names(track.artists.iter().map(|a| a.name.as_str())),
            track.image(art_px).map(str::to_string),
            false,
            track.explicit,
        ),
        Row::Playlist(playlist) => (
            playlist.uri.clone(),
            playlist.name.clone(),
            gettext(locale, "Playlist"),
            playlist.owner_name().to_string(),
            pick_image(&playlist.images, art_px).map(str::to_string),
            false,
            false,
        ),
        Row::Album(album) => (
            album.uri.clone(),
            album.name.clone(),
            gettext(locale, "Album"),
            join_names(album.artists.iter().map(|a| a.name.as_str())),
            pick_image(&album.images, art_px).map(str::to_string),
            false,
            false,
        ),
    };
    let id = Id::new(("mcloud-top-result", &uri));
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    if let Row::Artist(artist) = item {
        widgets::prefetch_artist_on_hover(ui.ctx(), &response, &artist.id, &mut app.actions);
    }
    let hovered = ui.rect_contains_pointer(rect);
    let song = matches!(item, Row::Song(_));
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(12),
        palette
            .text
            .gamma_multiply(if hovered { 0.1 } else { 0.071 }),
    );
    let art = Rect::from_center_size(
        pos2(rect.left() + 14.0 + TOP_ART / 2.0, rect.center().y),
        Vec2::splat(TOP_ART),
    );
    let radius = if round { TOP_ART / 2.0 } else { 5.0 };
    widgets::paint_cover(
        ui,
        &palette,
        image.as_deref(),
        art,
        radius,
        if round { Icon::User } else { Icon::Music },
        Some(app.backend.art()),
    );
    if song && hovered {
        dim_with_play(ui, art, radius);
    }
    let end_room = if song {
        10.0 + 28.0 + 4.0
    } else {
        14.0 + 14.0 + 8.0
    };
    let text_left = art.right() + 12.0;
    let text_width = (rect.right() - end_room - text_left).max(20.0);
    let top = rect.center().y - 19.0;
    let playing = song && app.now_playing().is_some_and(|now| now.uri == uri);
    let name_colour = if playing {
        palette.accent
    } else {
        palette.text
    };
    name_line(
        ui,
        &name,
        explicit,
        theme::regular(15.0),
        name_colour,
        pos2(text_left, top),
        text_width,
        palette.window,
        palette.text,
    );
    let sub = if by.is_empty() {
        kind.to_string()
    } else {
        format!("{kind} · {by}")
    };
    let sub_colour = palette.text.gamma_multiply(0.6);
    let subtitle = widgets::ellipsized(ui, &sub, theme::regular(13.0), sub_colour, text_width, 1);
    ui.painter()
        .galley(pos2(text_left, top + 21.0), subtitle, sub_colour);

    let menu_id = ui.make_persistent_id(("mcloud-top-menu", &uri));
    let mut more_clicked = false;
    if song {
        let more = Rect::from_center_size(
            pos2(rect.right() - 10.0 - 14.0, rect.center().y),
            Vec2::splat(28.0),
        );
        more_clicked = more_button(app, ui, more, id.with("more"), menu_id);
    } else {
        let chevron = Rect::from_center_size(
            pos2(rect.right() - 14.0 - 7.0, rect.center().y),
            Vec2::splat(14.0),
        );
        theme::paint_icon(ui, Icon::McloudChevron, chevron, 14.0, sub_colour);
    }
    theme::focus_ring(ui, &response);
    if response.clicked() && !more_clicked {
        match item {
            // A search result plays on its own; Spotify continues after it.
            Row::Song(_) => app.actions.push(Action::PlayUris {
                uris: vec![uri.clone()],
                index: 0,
            }),
            Row::Artist(artist) => app
                .actions
                .push(Action::Open(Page::Artist(artist.id.clone()))),
            Row::Playlist(playlist) => app
                .actions
                .push(Action::Open(Page::Playlist(playlist.id.clone()))),
            Row::Album(album) => app
                .actions
                .push(Action::Open(Page::Album(album.id.clone()))),
        }
    }
    let menu = egui::Popup::context_menu(&response)
        .id(menu_id)
        .frame(widgets::menu_frame(&palette));
    match item {
        Row::Song(track) => {
            let item = PlayableItem::Track((*track).clone());
            menu.show(|ui| widgets::item_menu(ui, app, &item, None, None));
        }
        _ => {
            menu.show(|ui| widgets::context_menu_items(ui, app, &uri, &name, None));
        }
    }
}

/// The artwork dimmed under a song's play glyph, on hover.
fn dim_with_play(ui: &egui::Ui, art: Rect, radius: f32) {
    ui.painter().rect_filled(
        art,
        CornerRadius::same(radius as u8),
        Color32::from_black_alpha(102),
    );
    let glyph = Rect::from_center_size(art.center(), Vec2::splat(16.0));
    theme::paint_icon(ui, Icon::PlayFilled, glyph, 16.0, Color32::WHITE);
}

/// A title on one line, then Apple's explicit mark: a small square in the
/// text's grey with the page's colour for the letter.
#[expect(clippy::too_many_arguments)]
fn name_line(
    ui: &egui::Ui,
    name: &str,
    explicit: bool,
    font: egui::FontId,
    colour: Color32,
    at: egui::Pos2,
    width: f32,
    page: Color32,
    ink: Color32,
) {
    let room = if explicit { width - 17.0 } else { width };
    let galley = widgets::ellipsized(ui, name, font, colour, room.max(10.0), 1);
    let end = at.x + galley.size().x;
    let height = galley.size().y;
    ui.painter().galley(at, galley, colour);
    if explicit {
        let mark = Rect::from_center_size(
            pos2(end + 5.0 + 6.0, at.y + height / 2.0),
            Vec2::splat(12.0),
        );
        ui.painter()
            .rect_filled(mark, CornerRadius::same(3), ink.gamma_multiply(0.45));
        ui.painter().text(
            mark.center(),
            egui::Align2::CENTER_CENTER,
            "E",
            theme::bold(9.0),
            page,
        );
    }
}

/// A "…" that opens the menu `menu_id` names; true when it was clicked.
fn more_button(app: &App, ui: &egui::Ui, rect: Rect, id: Id, menu_id: Id) -> bool {
    let palette = app.palette;
    let response = ui.interact(rect, id, Sense::click());
    let label = gettext(app.locale, "More options");
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    let colour = if response.hovered() {
        palette.accent
    } else {
        palette.text.gamma_multiply(0.753)
    };
    theme::paint_icon(ui, Icon::Ellipsis, rect, 16.0, colour);
    theme::focus_ring(ui, &response);
    let clicked = response.clicked();
    if clicked {
        egui::Popup::open_id(ui.ctx(), menu_id);
    }
    clicked
}

/// A song in the Songs shelf: 56pt, 48pt artwork, a rule from the text on
/// under all but a column's last.
fn song_row(app: &mut App, ui: &mut egui::Ui, rect: Rect, track: &Track, rule: bool) {
    let palette = app.palette;
    let art_px = app.art_px(SONG_ART);
    let id = Id::new(("mcloud-search-song", &track.uri));
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &track.name));
    let hovered = ui.rect_contains_pointer(rect);
    let art = Rect::from_center_size(
        pos2(rect.left() + SONG_ART / 2.0, rect.center().y),
        Vec2::splat(SONG_ART),
    );
    widgets::paint_cover(
        ui,
        &palette,
        track.image(art_px),
        art,
        5.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    if hovered {
        dim_with_play(ui, art, 5.0);
    }
    let text_left = art.right() + 12.0;
    let text_width = (rect.right() - 34.0 - text_left).max(20.0);
    let top = rect.center().y - 19.0;
    let playing = app.now_playing().is_some_and(|now| now.uri == track.uri);
    name_line(
        ui,
        &track.name,
        track.explicit,
        theme::regular(15.0),
        if playing {
            palette.accent
        } else {
            palette.text
        },
        pos2(text_left, top),
        text_width,
        palette.window,
        palette.text,
    );
    let artists = join_names(track.artists.iter().map(|a| a.name.as_str()));
    let sub_colour = palette.text.gamma_multiply(0.6);
    let subtitle = widgets::ellipsized(
        ui,
        &artists,
        theme::regular(13.0),
        sub_colour,
        text_width,
        1,
    );
    ui.painter()
        .galley(pos2(text_left, top + 21.0), subtitle, sub_colour);
    if rule {
        ui.painter().hline(
            text_left..=rect.right(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, palette.text.gamma_multiply(0.11)),
        );
    }
    finish_song(app, ui, rect, &response, track, id);
}

/// What a song row does: "…", play on click, and its menu.
fn finish_song(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    track: &Track,
    id: Id,
) {
    let palette = app.palette;
    let menu_id = ui.make_persistent_id(("mcloud-song-menu", &track.uri));
    let more = Rect::from_center_size(
        pos2(rect.right() - 14.0, rect.center().y),
        Vec2::splat(28.0),
    );
    let more_clicked = more_button(app, ui, more, id.with("more"), menu_id);
    theme::focus_ring(ui, response);
    if response.clicked() && !more_clicked {
        app.actions.push(Action::PlayUris {
            uris: vec![track.uri.clone()],
            index: 0,
        });
    }
    let item = PlayableItem::Track(track.clone());
    egui::Popup::context_menu(response)
        .id(menu_id)
        .frame(widgets::menu_frame(&palette))
        .show(|ui| widgets::item_menu(ui, app, &item, None, None));
}

/// A kind's page: a large title, then the table or the grid.
fn kind_page(app: &mut App, ui: &mut egui::Ui, results: &SearchResults, kind: SearchFilter) {
    let palette = app.palette;
    let title = match kind {
        SearchFilter::Podcasts => gettext(app.locale, "Podcasts & Shows"),
        kind => kind.label(app.locale),
    };
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add_space(SIDE);
        theme::text(ui, title.as_ref(), theme::bold(28.0), palette.text);
    });
    ui.add_space(14.0 - ui.spacing().item_spacing.y);
    let counts = shelves(ui.available_width());
    match kind {
        SearchFilter::Songs => {
            let tracks = results
                .tracks
                .as_ref()
                .map(|page| page.items.as_slice())
                .unwrap_or_default();
            song_table(app, ui, tracks);
        }
        SearchFilter::Artists => {
            let items = results
                .artists
                .as_ref()
                .map(|page| page.items.as_slice())
                .unwrap_or_default();
            grid(ui, items, counts.artist, counts.gap, |ui, artist, art| {
                search::artist_card(app, ui, artist, Some(art));
            });
        }
        SearchFilter::Albums => {
            let items = results
                .albums
                .as_ref()
                .map(|page| page.items.as_slice())
                .unwrap_or_default();
            grid(ui, items, counts.album, counts.gap, |ui, album, art| {
                search::album_card(app, ui, album, Some(art));
            });
        }
        SearchFilter::Playlists => {
            let items = results
                .playlists
                .as_ref()
                .map(|page| page.items.as_slice())
                .unwrap_or_default();
            grid(ui, items, counts.album, counts.gap, |ui, playlist, art| {
                search::playlist_card(app, ui, playlist, Some(art));
            });
        }
        SearchFilter::Podcasts => {
            let items = results
                .shows
                .as_ref()
                .map(|page| page.items.as_slice())
                .unwrap_or_default();
            grid(ui, items, counts.album, counts.gap, |ui, show, art| {
                search::show_card(app, ui, show, Some(art));
            });
        }
        SearchFilter::Episodes => {
            ui.horizontal_top(|ui| {
                ui.add_space(SIDE);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - SIDE);
                    search::episodes(app, ui, results, usize::MAX);
                });
            });
        }
        SearchFilter::All => {}
    }
    // The next page comes as this one nears its end.
    if results.has_more(kind) {
        let (end, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        if ui.is_rect_visible(end.expand2(vec2(0.0, 800.0))) {
            app.actions.push(Action::SearchMore(kind));
        }
        if app.search.more_pending {
            ui.horizontal(|ui| {
                ui.add_space(SIDE);
                widgets::loading_row(ui, &palette, app.locale);
            });
        }
    }
}

/// The shelf's cards in rows of as many as the shelf shows.
fn grid<T>(
    ui: &mut egui::Ui,
    items: &[T],
    count: usize,
    gap: f32,
    mut card: impl FnMut(&mut egui::Ui, &T, f32),
) {
    let inner = ui.available_width() - 2.0 * SIDE;
    let art = (inner - gap * (count as f32 - 1.0)) / count as f32;
    for row in items.chunks(count) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space(SIDE);
            for item in row {
                card(ui, item, art);
            }
        });
        ui.add_space(24.0 - 8.0 - ui.spacing().item_spacing.y);
    }
}

/// The columns of Apple's Songs table, from the left edge of the table:
/// Song, Artist, Album, Time and "…".
fn table_columns(width: f32) -> [f32; 5] {
    const TIME: f32 = 56.0;
    const MORE: f32 = 28.0;
    let room = (width - TIME - MORE).max(0.0);
    let unit = room / (1.4 + 1.0 + 1.15);
    [0.0, unit * 1.4, unit * 2.4, unit * 3.55, unit * 3.55 + TIME]
}

/// Apple's Songs table: hairline column marks in the header, a rule under
/// each row; the song plays, the artist and the album open.
fn song_table(app: &mut App, ui: &mut egui::Ui, tracks: &[Track]) {
    let palette = app.palette;
    let locale = app.locale;
    let width = ui.available_width() - 2.0 * SIDE;
    let columns = table_columns(width);
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), TABLE_HEAD), Sense::hover());
    let left = head.left() + SIDE;
    let right = left + width;
    ui.painter().hline(
        left..=right,
        head.bottom() - 0.5,
        egui::Stroke::new(1.0, palette.text.gamma_multiply(0.11)),
    );
    let head_colour = palette.text.gamma_multiply(0.6);
    let heads = [
        pgettext(locale, "search result", "Song"),
        gettext(locale, "Artist"),
        gettext(locale, "Album"),
    ];
    for (index, label) in heads.iter().enumerate() {
        let inset = if index == 0 { 0.0 } else { 12.0 };
        let x = left + columns[index];
        ui.painter().text(
            pos2(x + inset, head.center().y),
            egui::Align2::LEFT_CENTER,
            label.as_ref(),
            theme::semibold(12.0),
            head_colour,
        );
        if index > 0 {
            ui.painter().vline(
                x,
                (head.center().y - 7.0)..=(head.center().y + 7.0),
                egui::Stroke::new(1.0, palette.text.gamma_multiply(0.16)),
            );
        }
    }
    ui.painter().vline(
        left + columns[3],
        (head.center().y - 7.0)..=(head.center().y + 7.0),
        egui::Stroke::new(1.0, palette.text.gamma_multiply(0.16)),
    );
    ui.painter().text(
        pos2(left + columns[4] - 6.0, head.center().y),
        egui::Align2::RIGHT_CENTER,
        gettext(locale, "Time").as_ref(),
        theme::semibold(12.0),
        head_colour,
    );
    let art_px = app.art_px(TABLE_ART);
    for track in tracks {
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), SONG_ROW), Sense::hover());
        if !ui.is_rect_visible(row) {
            continue;
        }
        let rect = Rect::from_min_max(pos2(left, row.top()), pos2(right, row.bottom()));
        ui.painter().hline(
            left..=right,
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, palette.text.gamma_multiply(0.08)),
        );
        let id = Id::new(("mcloud-search-table", &track.uri));
        let song_cell = Rect::from_min_max(rect.min, pos2(left + columns[1], rect.bottom()));
        let response = ui.interact(song_cell, id, Sense::click());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &track.name));
        let art = Rect::from_center_size(
            pos2(rect.left() + TABLE_ART / 2.0, rect.center().y),
            Vec2::splat(TABLE_ART),
        );
        widgets::paint_cover(
            ui,
            &palette,
            track.image(art_px),
            art,
            4.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        if ui.rect_contains_pointer(song_cell) {
            dim_with_play(ui, art, 4.0);
        }
        let text_left = art.right() + 12.0;
        let playing = app.now_playing().is_some_and(|now| now.uri == track.uri);
        let name_font = theme::regular(13.0);
        name_line(
            ui,
            &track.name,
            track.explicit,
            name_font,
            if playing {
                palette.accent
            } else {
                palette.text
            },
            pos2(text_left, rect.center().y - 8.5),
            (song_cell.right() - text_left - 8.0).max(20.0),
            palette.window,
            palette.text,
        );
        let artists = join_names(track.artists.iter().map(|a| a.name.as_str()));
        let first_artist = track.artists.first().and_then(|artist| artist.id.clone());
        if table_link(
            app,
            ui,
            id.with("artist"),
            &artists,
            left + columns[1] + 12.0,
            left + columns[2] - 8.0,
            rect.center().y,
        ) && let Some(artist) = first_artist
        {
            app.actions.push(Action::Open(Page::Artist(artist)));
        }
        if let Some(album) = &track.album
            && table_link(
                app,
                ui,
                id.with("album"),
                &album.name,
                left + columns[2] + 12.0,
                left + columns[3] - 8.0,
                rect.center().y,
            )
        {
            app.actions
                .push(Action::Open(Page::Album(album.id.clone())));
        }
        ui.painter().text(
            pos2(left + columns[4] - 6.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            crate::util::format_duration_ms(track.duration_ms),
            theme::regular(13.0),
            palette.text.gamma_multiply(0.6),
        );
        let menu_id = ui.make_persistent_id(("mcloud-table-menu", &track.uri));
        let more = Rect::from_center_size(pos2(right - 14.0, rect.center().y), Vec2::splat(28.0));
        let more_clicked = more_button(app, ui, more, id.with("more"), menu_id);
        theme::focus_ring(ui, &response);
        if response.clicked() && !more_clicked {
            app.actions.push(Action::PlayUris {
                uris: vec![track.uri.clone()],
                index: 0,
            });
        }
        let item = PlayableItem::Track(track.clone());
        egui::Popup::context_menu(&response)
            .id(menu_id)
            .frame(widgets::menu_frame(&palette))
            .show(|ui| widgets::item_menu(ui, app, &item, None, None));
    }
}

/// A cell that opens what it names: quiet until the pointer is on it.
fn table_link(
    app: &App,
    ui: &egui::Ui,
    id: Id,
    text: &str,
    left: f32,
    right: f32,
    center_y: f32,
) -> bool {
    let palette = app.palette;
    let width = (right - left).max(10.0);
    let font = theme::regular(13.0);
    let galley = widgets::ellipsized(ui, text, font, palette.text, width, 1);
    let size = galley.size();
    let rect = Rect::from_min_size(pos2(left, center_y - size.y / 2.0), size);
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, text));
    let colour = if response.hovered() {
        palette.text
    } else {
        palette.text.gamma_multiply(0.7)
    };
    ui.painter().galley(rect.min, galley, colour);
    if response.hovered() {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 1.0,
            egui::Stroke::new(1.0, colour),
        );
    }
    theme::focus_ring(ui, &response);
    response.clicked()
}

/// One entry of Top Results.
enum Row<'a> {
    Artist(&'a Artist),
    Song(&'a Track),
    Playlist(&'a Playlist),
    Album(&'a Album),
}

/// Songs and collections in turn, as Spotify alternates them: a song, a
/// playlist, a song, an album, until the kinds run out.
fn mix<'a>(songs: &'a [Track], playlists: &'a [Playlist], albums: &'a [Album]) -> Vec<Row<'a>> {
    let mut songs = songs.iter().map(Row::Song);
    let mut playlists = playlists.iter().map(Row::Playlist);
    let mut albums = albums.iter().map(Row::Album);
    let mut rows = Vec::new();
    let mut turn = 0;
    loop {
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
            None => return rows,
        }
        turn += 1;
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
            // The mix holds songs and collections; suggestions add artists apart.
            Row::Artist(_) => {}
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
                Row::Artist(_) => 'r',
                Row::Song(_) => 's',
                Row::Playlist(_) => 'p',
                Row::Album(_) => 'a',
            })
            .collect();
        assert_eq!(kinds.iter().collect::<String>(), "spsasa");
    }

    /// Apple's counts at the widths measured in Music, and what a row of each
    /// leaves: the next card starts at the right margin.
    #[test]
    fn shelves_follow_the_width_beside_the_sidebar() {
        let counts = |w| {
            let s = shelves(w);
            (s.gap, s.top, s.album, s.artist, s.song)
        };
        assert_eq!(counts(600.0), (16.0, 2, 3, 4, 1));
        assert_eq!(counts(800.0), (16.0, 3, 4, 5, 2));
        assert_eq!(counts(1180.0), (20.0, 4, 5, 6, 3));
        assert_eq!(counts(1300.0), (20.0, 4, 6, 7, 3));
        assert_eq!(counts(1500.0), (20.0, 4, 7, 8, 4));
    }

    /// A row's first card starts on the page's margin, in a shelf and a grid.
    #[test]
    fn rows_start_on_the_pages_margin() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let left = ui.cursor().left();
            let mut first = None;
            shelf_row(ui, "row", 100.0, 16.0, |ui| {
                first = Some(ui.cursor().left())
            });
            assert_eq!(first, Some(left + SIDE));
            let mut first = None;
            grid(ui, &[()], 3, 16.0, |ui, (), _| {
                first = Some(ui.cursor().left());
            });
            assert_eq!(first, Some(left + SIDE));
        });
        output.textures_delta.clear();
    }

    /// Top Results leads with the best artist and fills two rows at most.
    #[test]
    fn top_results_lead_with_the_artist_and_stop_at_the_count() {
        fn page<T>(items: Vec<T>) -> crate::api::models::Page<T> {
            crate::api::models::Page {
                items,
                total: 0,
                limit: 0,
                offset: 0,
                next: None,
            }
        }
        let results = SearchResults {
            artists: Some(page(vec![Artist::default(), Artist::default()])),
            tracks: Some(page(vec![Track::default(); 5])),
            albums: Some(page(vec![Album::default(); 5])),
            ..Default::default()
        };
        let kinds = top_items(&results, 4);
        assert_eq!(kinds.len(), 4);
        assert!(matches!(kinds[0], Row::Artist(_)));
        assert!(matches!(kinds[1], Row::Song(_)));
    }

    /// A kind's page survives being saved and reopened.
    #[test]
    fn a_kinds_page_round_trips() {
        for kind in SearchFilter::ALL {
            let page = Page::SearchKind(kind);
            assert_eq!(Page::decode(&page.encode()), Some(page));
        }
        assert_eq!(Page::decode("search:nope"), None);
    }

    /// The table's fractions fill what the time and "…" leave.
    #[test]
    fn the_song_table_splits_its_width_as_apples() {
        let [song, artist, album, time, more] = table_columns(1084.0);
        assert_eq!(song, 0.0);
        assert!((artist - 394.4).abs() < 0.5);
        assert!((album - 676.1).abs() < 0.5);
        assert!((time - 1000.0).abs() < 0.5);
        assert!((more - 1056.0).abs() < 0.5);
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

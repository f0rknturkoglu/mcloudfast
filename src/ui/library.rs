//! Full-page library grids: albums, artists, podcasts, episodes.

use crate::api::models::{join_names, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

use super::widgets;

pub fn show(app: &mut App, ui: &mut egui::Ui, page: Page) {
    let palette = app.palette;
    ui.add_space(8.0);
    let locale = app.locale;
    let (title, empty_title, empty_body) = match page {
        Page::RecentlyAdded => (
            gettext(locale, "Recently Added"),
            gettext(locale, "Nothing added yet"),
            gettext(locale, "Recently added music appears here."),
        ),
        Page::AllPlaylists => (
            gettext(locale, "All Playlists"),
            gettext(locale, "No playlists yet"),
            gettext(locale, "Your playlists appear here."),
        ),
        Page::Albums => (
            gettext(locale, "Albums"),
            gettext(locale, "No saved albums"),
            gettext(locale, "Saved albums appear here."),
        ),
        Page::Artists => (
            gettext(locale, "Artists"),
            gettext(locale, "No followed artists"),
            gettext(locale, "Followed artists appear here."),
        ),
        Page::Podcasts => (
            gettext(locale, "Podcasts"),
            gettext(locale, "No podcasts yet"),
            gettext(locale, "Followed podcasts appear here."),
        ),
        _ => (
            gettext(locale, "Episodes"),
            gettext(locale, "No saved episodes"),
            gettext(locale, "Saved episodes appear here."),
        ),
    };
    if crate::ui::mcloud::active(ui.ctx())
        && let Some(entries) = mcloud_entries(app, &page)
    {
        super::mcloud::library::show(app, ui, &page, &title, &entries);
        let (loading, error, can_load, icon) = match page {
            Page::Artists => {
                let list = &app.library.artists;
                (
                    list.loading,
                    list.error.clone(),
                    list.can_load_more(),
                    Icon::Users,
                )
            }
            Page::AllPlaylists => (
                app.library.playlists_loading_since.is_some()
                    && app.library.playlists.get().is_none(),
                None,
                false,
                Icon::ListMusic,
            ),
            _ => {
                let list = &app.library.albums;
                let loading = list.loading
                    || (page == Page::RecentlyAdded
                        && app.library.playlists_loading_since.is_some()
                        && app.library.playlists.get().is_none());
                (
                    list.loading || loading,
                    list.error.clone(),
                    list.can_load_more(),
                    Icon::Disc,
                )
            }
        };
        let empty = entries.is_empty() && !loading;
        footer(
            app,
            ui,
            page,
            loading,
            error,
            can_load,
            empty,
            &empty_title,
            &empty_body,
            icon,
        );
        return;
    }
    theme::text(ui, title.as_ref(), theme::bold(28.0), palette.text);
    ui.add_space(14.0);
    match page {
        Page::RecentlyAdded => {
            let card_art = app.art_px(widgets::CARD_ART);
            let mut items: Vec<RecentItem> = Vec::new();
            for saved in &app.library.albums.items {
                let album = &saved.album;
                let subtitle = join_names(album.artists.iter().map(|artist| artist.name.as_str()));
                items.push(RecentItem {
                    uri: album.uri.clone(),
                    name: album.name.clone(),
                    subtitle,
                    image: pick_image(&album.images, card_art).map(str::to_string),
                    page: Page::Album(album.id.clone()),
                    added_at: saved.added_at.clone(),
                });
            }
            if let Some(playlists) = app.library.playlists.get() {
                for pl in playlists {
                    items.push(RecentItem {
                        uri: pl.uri.clone(),
                        name: pl.name.clone(),
                        subtitle: pl.owner_name().to_string(),
                        image: pick_image(&pl.images, card_art).map(str::to_string),
                        page: Page::Playlist(pl.id.clone()),
                        added_at: None,
                    });
                }
            }
            items.sort_by(|a, b| b.added_at.cmp(&a.added_at));

            let count = items.len();
            let card_height = widgets::card_row_height(ui);
            widgets::virtual_wrapped_cards(ui, count, card_height, |ui, index| {
                let item = &items[index];
                let uri = item.uri.clone();
                let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card = widgets::card(
                    ui,
                    app,
                    item.image.as_deref(),
                    &item.name,
                    &item.subtitle,
                    widgets::CardCover::square(playing_here),
                );
                if card.play {
                    if context_here {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri: uri.clone(),
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
                if card.clicked {
                    app.actions.push(Action::Open(item.page.clone()));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("library-recent-menu", &item.uri)),
                    &palette,
                )
                .show(|ui| widgets::context_menu_items(ui, app, &item.uri, &item.name, None));
            });
            let list = &app.library.albums;
            let loading = list.loading
                || (app.library.playlists_loading_since.is_some()
                    && app.library.playlists.get().is_none());
            let empty = items.is_empty() && list.loaded_once;
            footer(
                app,
                ui,
                page,
                loading,
                list.error.clone(),
                list.can_load_more(),
                empty,
                &empty_title,
                &empty_body,
                Icon::McloudClock,
            );
        }
        Page::AllPlaylists => {
            let playlists = app.library.playlists.get().cloned().unwrap_or_default();
            let count = playlists.len();
            let card_height = widgets::card_row_height(ui);
            widgets::virtual_wrapped_cards(ui, count, card_height, |ui, index| {
                let playlist = &playlists[index];
                let uri = playlist.uri.clone();
                let id = playlist.id.clone();
                let subtitle = playlist.owner_name();
                let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&playlist.images, card_art),
                    &playlist.name,
                    subtitle,
                    widgets::CardCover::square(playing_here),
                );
                if card.play {
                    if context_here {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri: uri.clone(),
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
                if card.clicked {
                    app.actions.push(Action::Open(Page::Playlist(id)));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("library-playlist-menu", &playlist.uri)),
                    &palette,
                )
                .show(|ui| {
                    let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
                    widgets::context_menu_items(
                        ui,
                        app,
                        &playlist.uri,
                        &playlist.name,
                        owned.then_some(playlist),
                    );
                });
            });
            let loading = app.library.playlists_loading_since.is_some()
                && app.library.playlists.get().is_none();
            let empty = playlists.is_empty() && !loading;
            footer(
                app,
                ui,
                page,
                loading,
                None,
                false,
                empty,
                &empty_title,
                &empty_body,
                Icon::ListMusic,
            );
        }
        Page::Albums => {
            let card_height = widgets::card_row_height(ui);
            let count = app.library.albums.items.len();
            widgets::virtual_wrapped_cards(ui, count, card_height, |ui, index| {
                let album = app.library.albums.items[index].album.clone();
                let id = album.id.clone();
                let uri = album.uri.clone();
                let subtitle = join_names(album.artists.iter().map(|artist| artist.name.as_str()));
                let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
                let playing_here = context_here && app.believed_playing();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&album.images, card_art),
                    &album.name,
                    &subtitle,
                    widgets::CardCover::square(playing_here),
                );
                if card.play {
                    if context_here {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri,
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
                if card.clicked {
                    app.actions.push(Action::Open(Page::Album(id)));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("library-album-menu", &album.uri)),
                    &palette,
                )
                .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
            });
            let list = &app.library.albums;
            let (loading, error, can_load, empty) = (
                list.loading,
                list.error.clone(),
                list.can_load_more(),
                list.items.is_empty() && list.loaded_once,
            );
            footer(
                app,
                ui,
                page,
                loading,
                error,
                can_load,
                empty,
                &empty_title,
                &empty_body,
                Icon::Disc,
            );
        }
        Page::Artists => {
            let card_height = widgets::card_row_height(ui);
            let count = app.library.artists.items.len();
            let artist_label = gettext(locale, "Artist");
            widgets::virtual_wrapped_cards(ui, count, card_height, |ui, index| {
                let artist = app.library.artists.items[index].clone();
                let id = artist.id.clone();
                let uri = artist.uri.clone();
                let context_here = app.playing_context_uri().as_deref() == Some(uri.as_str());
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
                widgets::prefetch_artist_on_hover(ui.ctx(), &card.response, &id, &mut app.actions);
                if card.play {
                    if context_here {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri,
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
                if card.clicked {
                    app.actions.push(Action::Open(Page::Artist(id)));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("library-artist-menu", &artist.uri)),
                    &palette,
                )
                .show(|ui| widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None));
            });
            let list = &app.library.artists;
            let (loading, error, can_load, empty) = (
                list.loading,
                list.error.clone(),
                list.can_load_more(),
                list.items.is_empty() && list.loaded_once,
            );
            footer(
                app,
                ui,
                page,
                loading,
                error,
                can_load,
                empty,
                &empty_title,
                &empty_body,
                Icon::Users,
            );
        }
        Page::Podcasts => {
            let card_height = widgets::card_row_height(ui);
            let count = app.library.shows.items.len();
            widgets::virtual_wrapped_cards(ui, count, card_height, |ui, index| {
                let show = app.library.shows.items[index].show.clone();
                let id = show.id.clone();
                let card_art = app.art_px(widgets::CARD_ART);
                let card = widgets::card(
                    ui,
                    app,
                    pick_image(&show.images, card_art),
                    &show.name,
                    &show.publisher,
                    widgets::CardCover::default(),
                );
                if card.clicked {
                    app.actions.push(Action::Open(Page::Show(id)));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("library-show-menu", &show.uri)),
                    &palette,
                )
                .show(|ui| widgets::context_menu_items(ui, app, &show.uri, &show.name, None));
            });
            let list = &app.library.shows;
            let (loading, error, can_load, empty) = (
                list.loading,
                list.error.clone(),
                list.can_load_more(),
                list.items.is_empty() && list.loaded_once,
            );
            footer(
                app,
                ui,
                page,
                loading,
                error,
                can_load,
                empty,
                &empty_title,
                &empty_body,
                Icon::Mic,
            );
        }
        _ => {
            let episodes: Vec<_> = app
                .library
                .episodes
                .items
                .iter()
                .map(|saved| saved.episode.clone())
                .collect();
            widgets::virtual_rows(
                ui,
                episodes.len(),
                super::show::EPISODE_ROW_HEIGHT,
                |ui, index| {
                    super::show::episode_row(app, ui, &episodes[index], None);
                },
            );
            let list = &app.library.episodes;
            let (loading, error, can_load, empty) = (
                list.loading,
                list.error.clone(),
                list.can_load_more(),
                list.items.is_empty() && list.loaded_once,
            );
            footer(
                app,
                ui,
                page,
                loading,
                error,
                can_load,
                empty,
                &empty_title,
                &empty_body,
                Icon::Bookmark,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn footer(
    app: &mut App,
    ui: &mut egui::Ui,
    page: Page,
    loading: bool,
    error: Option<String>,
    can_load: bool,
    empty: bool,
    empty_title: &str,
    empty_body: &str,
    icon: Icon,
) {
    let palette = app.palette;
    if !app.network_reachable && empty {
        widgets::empty_state(
            ui,
            &palette,
            Icon::WifiOff,
            &gettext(app.locale, "You're offline"),
            &gettext(
                app.locale,
                "Spotifast will reconnect automatically when your connection returns.",
            ),
        );
        return;
    }
    if loading {
        ui.add_space(8.0);
        widgets::loading_row(ui, &palette, app.locale);
    }
    if let Some(error) = error {
        widgets::error_row(ui, app, &error, Some(page.clone()));
    }
    if empty && !loading {
        widgets::empty_state(ui, &palette, icon, empty_title, empty_body);
    }
    widgets::load_more_when_near_end(ui, app, page, can_load && !loading);
}

struct RecentItem {
    uri: String,
    name: String,
    subtitle: String,
    image: Option<String>,
    page: Page,
    added_at: Option<String>,
}

/// The covers of mcloud's library views, in Spotify's library order:
/// Recently Added mixes albums and playlists, newest first.
fn mcloud_entries(app: &App, page: &Page) -> Option<Vec<super::mcloud::library::Entry>> {
    use super::mcloud::library::Entry;
    let art = app.art_px(160.0);
    let album = |saved: &crate::api::models::SavedAlbum| {
        let album = &saved.album;
        Entry {
            uri: album.uri.clone(),
            name: album.name.clone(),
            subtitle: join_names(album.artists.iter().map(|artist| artist.name.as_str())),
            image: pick_image(&album.images, art).map(str::to_string),
            page: Page::Album(album.id.clone()),
            round: false,
        }
    };
    let playlist = |playlist: &crate::api::models::Playlist| Entry {
        uri: playlist.uri.clone(),
        name: playlist.name.clone(),
        subtitle: playlist.owner_name().to_string(),
        image: pick_image(&playlist.images, art).map(str::to_string),
        page: Page::Playlist(playlist.id.clone()),
        round: false,
    };
    let playlists = app
        .library
        .playlists
        .get()
        .map(Vec::as_slice)
        .unwrap_or_default();
    Some(match page {
        Page::Albums => app.library.albums.items.iter().map(album).collect(),
        Page::Artists => {
            let label = gettext(app.locale, "Artist").into_owned();
            app.library
                .artists
                .items
                .iter()
                .map(|artist| Entry {
                    uri: artist.uri.clone(),
                    name: artist.name.clone(),
                    subtitle: label.clone(),
                    image: pick_image(&artist.images, art).map(str::to_string),
                    page: Page::Artist(artist.id.clone()),
                    round: true,
                })
                .collect()
        }
        Page::AllPlaylists => playlists.iter().map(playlist).collect(),
        Page::RecentlyAdded => {
            let mut dated: Vec<(Option<&str>, Entry)> = app
                .library
                .albums
                .items
                .iter()
                .map(|saved| (saved.added_at.as_deref(), album(saved)))
                .chain(playlists.iter().map(|item| (None, playlist(item))))
                .collect();
            // Newest first; playlists, which carry no date, after.
            dated.sort_by(|a, b| b.0.cmp(&a.0));
            dated.into_iter().map(|(_, entry)| entry).collect()
        }
        _ => return None,
    })
}

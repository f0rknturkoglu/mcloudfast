//! Somebody's public profile: what a playlist's owner opens to, from the same
//! view Spotify's own clients read.

use egui::{Rect, Sense, Vec2, vec2};

use crate::app::App;
use crate::i18n::{gettext, ngettext};
use crate::model::{Action, Loadable, Page};
use crate::session_reads::UserPage;
use crate::theme::{self, Icon};
use crate::ui::widgets;
use crate::util;

/// The picture's side.
const AVATAR: f32 = 160.0;
/// The artwork's side on the cards, as mcloud's shelves set it.
const CARD_ART: Option<f32> = Some(180.0);

pub fn show(app: &mut App, ui: &mut egui::Ui, id: &str) {
    app.request_user_page(id, false);
    match app.user_pages.get(id).cloned() {
        Some(Loadable::Loaded(page)) => loaded(app, ui, id, &page),
        Some(Loadable::Failed(error)) => {
            ui.add_space(40.0);
            widgets::error_row(ui, app, &error, Some(Page::User(id.to_string())));
        }
        _ => {
            ui.add_space(40.0);
            ui.add(egui::Spinner::new().color(app.palette.secondary));
        }
    }
}

fn loaded(app: &mut App, ui: &mut egui::Ui, id: &str, page: &UserPage) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(28.0);
    let name = page.name.as_deref().unwrap_or(id);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(AVATAR), Sense::hover());
        let art = app.backend.art();
        widgets::paint_cover(
            ui,
            &palette,
            page.image_url.as_deref(),
            rect,
            AVATAR / 2.0,
            Icon::User,
            Some(art),
        );
        let (text, _) = ui.allocate_exact_size(
            vec2((ui.available_width()).max(0.0), AVATAR),
            Sense::hover(),
        );
        let column = Rect::from_center_size(text.center(), vec2(text.width(), 60.0));
        let mut column = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(column)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        theme::text(&mut column, name, theme::bold(26.0), palette.text);
        column.add_space(4.0);
        let mut stats: Vec<String> = Vec::new();
        if let Some(total) = page.total_playlists.filter(|total| *total > 0) {
            stats.push(
                ngettext(
                    locale,
                    "{count} playlist",
                    "{count} playlists",
                    total as u32,
                )
                .replace("{count}", &util::format_count(total)),
            );
        }
        if let Some(followers) = page.followers {
            stats.push(
                ngettext(
                    locale,
                    "{count} follower",
                    "{count} followers",
                    followers as u32,
                )
                .replace("{count}", &util::format_count(followers)),
            );
        }
        theme::text(
            &mut column,
            stats.join(" • "),
            theme::regular(13.0),
            palette.secondary,
        );
    });
    if !page.playlists.is_empty() {
        ui.add_space(28.0);
        theme::text(
            ui,
            gettext(locale, "Playlists"),
            theme::bold(18.0),
            palette.text,
        );
        ui.add_space(10.0);
        widgets::grid(ui, |ui| {
            for playlist in &page.playlists {
                let card = widgets::card(
                    ui,
                    app,
                    playlist.image_url.as_deref(),
                    &playlist.name,
                    &playlist.owner_name,
                    widgets::CardCover::square(false).sized(CARD_ART),
                );
                if card.play {
                    app.actions.push(Action::PlayContext {
                        uri: playlist.uri.clone(),
                        offset_uri: None,
                        offset_index: None,
                    });
                }
                if card.clicked {
                    app.actions
                        .push(Action::Open(Page::Playlist(playlist.id.clone())));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("user-playlist-menu", &playlist.uri)),
                    &palette,
                )
                .show(|ui| {
                    widgets::context_menu_items(ui, app, &playlist.uri, &playlist.name, None)
                });
            }
        });
    }
    if !page.artists.is_empty() {
        ui.add_space(28.0);
        theme::text(
            ui,
            gettext(locale, "Artists"),
            theme::bold(18.0),
            palette.text,
        );
        ui.add_space(10.0);
        widgets::grid(ui, |ui| {
            for artist in &page.artists {
                let card = widgets::card(
                    ui,
                    app,
                    artist.image_url.as_deref(),
                    &artist.name,
                    "",
                    widgets::CardCover {
                        circular: true,
                        ..Default::default()
                    }
                    .sized(CARD_ART),
                );
                if card.clicked {
                    app.actions
                        .push(Action::Open(Page::Artist(artist.id.clone())));
                }
                widgets::card_context_menu(
                    ui,
                    &card,
                    ui.make_persistent_id(("user-artist-menu", &artist.uri)),
                    &palette,
                )
                .show(|ui| widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None));
            }
        });
    }
    ui.add_space(28.0);
}

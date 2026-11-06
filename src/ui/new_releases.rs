//! The New releases page: every song of the latest releases from the
//! artists the account follows, newest release first, played like a
//! playlist.

use std::sync::Arc;

use egui::{CornerRadius, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{PlayableItem, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable, Page, RowContext};
use crate::theme::{self, Icon};

use super::collection::{Table, remember_table_items, songs_and_duration, table, table_items_hit};
use super::widgets;

/// The feed under mcloud, as its What's New: a grid of frosted cards, the
/// cover on the left, the kind and year, the title in bold, the artists under
/// it, and the save star on the right.
fn mcloud_feed(app: &mut App, ui: &mut egui::Ui) {
    const CARD_MIN: f32 = 360.0;
    const GAP: f32 = 14.0;
    const PAD: f32 = 14.0;
    const COVER: f32 = 96.0;
    const STAR: f32 = 40.0;
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(12.0);
    theme::text(
        ui,
        gettext(locale, "What's New"),
        theme::bold(28.0),
        palette.text,
    );
    ui.add_space(14.0 - ui.spacing().item_spacing.y);
    app.ensure_release_feed();
    let albums = match &app.home.new_releases {
        Loadable::Loaded(albums) => albums.clone(),
        Loadable::Failed(error) => {
            let error = error.clone();
            widgets::error_row(ui, app, &error, Some(Page::NewReleases));
            return;
        }
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::loading_row(ui, &palette, locale);
            return;
        }
    };
    if albums.is_empty() {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Music,
            &gettext(locale, "No new releases yet"),
            &gettext(locale, "Follow artists to see their latest releases here."),
        );
        return;
    }
    app.request_contains(albums.iter().map(|album| album.uri.clone()).collect());
    let available = ui.available_width();
    let columns = (((available + GAP) / (CARD_MIN + GAP)).floor() as usize).max(1);
    let width = (available - GAP * (columns as f32 - 1.0)) / columns as f32;
    let height = PAD * 2.0 + COVER;
    let rows = albums.len().div_ceil(columns);
    let (block, _) = ui.allocate_exact_size(
        vec2(available, height * rows as f32 + GAP * (rows as f32 - 1.0)),
        Sense::hover(),
    );
    let art_px = app.art_px(COVER);
    for (index, album) in albums.iter().enumerate() {
        let rect = Rect::from_min_size(
            block.min
                + vec2(
                    (width + GAP) * (index % columns) as f32,
                    (height + GAP) * (index / columns) as f32,
                ),
            vec2(width, height),
        );
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let id = ui.id().with(("mcloud-feed", &album.uri));
        let response = ui.interact(rect, id, Sense::click());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &album.name));
        let hovered = ui.rect_contains_pointer(rect);
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(22),
            palette
                .text
                .gamma_multiply(if hovered { 0.085 } else { 0.055 }),
        );
        let cover = Rect::from_min_size(rect.min + Vec2::splat(PAD), Vec2::splat(COVER));
        widgets::paint_cover(
            ui,
            &palette,
            pick_image(&album.images, art_px),
            cover,
            10.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        ui.painter().rect_stroke(
            cover,
            CornerRadius::same(10),
            egui::Stroke::new(1.0, palette.text.gamma_multiply(0.1)),
            egui::StrokeKind::Outside,
        );
        let star = Rect::from_center_size(
            pos2(rect.right() - PAD - STAR / 2.0, rect.center().y),
            Vec2::splat(STAR),
        );
        let text_left = cover.right() + 16.0;
        let text_width = (star.left() - 16.0 - text_left).max(20.0);
        let kind = album.kind_label(locale);
        let meta = match album.year() {
            Some(year) => format!("{kind} • {year}"),
            None => kind.into_owned(),
        };
        let sub = palette.text.gamma_multiply(0.6);
        let meta = widgets::ellipsized(ui, &meta, theme::regular(13.0), sub, text_width, 1);
        let title = widgets::ellipsized(
            ui,
            &album.name,
            theme::bold(17.0),
            palette.text,
            text_width,
            1,
        );
        let artists = crate::api::models::join_names(album.artists.iter().map(|a| a.name.as_str()));
        let artists = widgets::ellipsized(ui, &artists, theme::regular(14.0), sub, text_width, 1);
        let total = meta.size().y + 1.0 + title.size().y + 2.0 + artists.size().y;
        let mut y = rect.center().y - total / 2.0;
        let (meta_height, title_height) = (meta.size().y, title.size().y);
        ui.painter().galley(pos2(text_left, y), meta, sub);
        y += meta_height + 1.0;
        ui.painter().galley(pos2(text_left, y), title, palette.text);
        y += title_height + 2.0;
        ui.painter().galley(pos2(text_left, y), artists, sub);

        let saved = app.is_saved(&album.uri).unwrap_or(false);
        let label = gettext(
            locale,
            if saved {
                "Remove from Your Library"
            } else {
                "Save to Your Library"
            },
        );
        let star_response = ui.interact(star, id.with("star"), Sense::click());
        star_response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        ui.painter().circle_filled(
            star.center(),
            STAR / 2.0,
            palette
                .text
                .gamma_multiply(if star_response.hovered() { 0.12 } else { 0.06 }),
        );
        ui.painter().circle_stroke(
            star.center(),
            STAR / 2.0 - 0.5,
            egui::Stroke::new(1.0, palette.text.gamma_multiply(0.1)),
        );
        theme::paint_icon(
            ui,
            if saved {
                Icon::McloudStarFilled
            } else {
                Icon::McloudStar
            },
            star,
            16.0,
            palette.accent,
        );
        theme::focus_ring(ui, &star_response);
        theme::focus_ring(ui, &response);
        if star_response.clicked() {
            app.actions.push(Action::ToggleSaved(album.uri.clone()));
        } else if response.clicked() {
            app.actions
                .push(Action::Open(Page::Album(album.id.clone())));
        }
        egui::Popup::context_menu(&response)
            .id(id.with("menu"))
            .frame(widgets::menu_frame(&palette))
            .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
    }
}

pub fn page(app: &mut App, ui: &mut egui::Ui) {
    if super::mcloud::active(ui.ctx()) && app.settings.mcloud_feed_cards {
        mcloud_feed(app, ui);
        return;
    }
    let palette = app.palette;
    let locale = app.locale;
    if app.home.new_release_tracks.get().is_none() && !app.home.new_release_tracks.is_loading() {
        app.ensure_loaded(Page::NewReleases);
    }
    ui.add_space(12.0);
    theme::text(
        ui,
        gettext(locale, "New releases"),
        theme::bold(30.0),
        palette.text,
    );
    ui.add_space(4.0);
    theme::text(
        ui,
        gettext(
            locale,
            "The latest releases from artists you follow, newest first.",
        ),
        theme::regular(13.5),
        palette.secondary,
    );
    ui.add_space(18.0);

    // A shelf that could not load leaves the page nothing to read.
    if let Loadable::Failed(error) = &app.home.new_releases
        && app.home.new_release_tracks.get().is_none()
    {
        let error = error.clone();
        widgets::error_row(ui, app, &error, Some(Page::NewReleases));
        return;
    }
    if app
        .home
        .new_releases
        .get()
        .is_some_and(|albums| albums.is_empty())
    {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Music,
            &gettext(locale, "No new releases"),
            &gettext(
                locale,
                "New albums and singles from artists you follow will appear here.",
            ),
        );
        return;
    }
    let tracks = match &app.home.new_release_tracks {
        Loadable::Loaded(tracks) => tracks,
        Loadable::Loading | Loadable::NotLoaded => {
            widgets::loading_row(ui, &palette, locale);
            return;
        }
        Loadable::Failed(error) => {
            let error = error.clone();
            widgets::error_row(ui, app, &error, Some(Page::NewReleases));
            return;
        }
    };
    let count = u32::try_from(tracks.len()).unwrap_or(u32::MAX);
    let duration = tracks
        .iter()
        .map(|track| u64::from(track.duration_ms))
        .sum();
    let generation = app.home.new_release_tracks_generation;
    let names = app.user_names_revision;
    let items = match table_items_hit(app, &Page::NewReleases, generation, generation, names) {
        Some(items) => items,
        None => {
            let rows = app
                .home
                .new_release_tracks
                .get()
                .into_iter()
                .flatten()
                .map(|track| (PlayableItem::Track(track.clone()), None, None))
                .collect();
            remember_table_items(app, Page::NewReleases, generation, generation, names, rows)
        }
    };
    let uris: Arc<[String]> = items
        .iter()
        .map(|(item, _, _)| item.uri().to_string())
        .collect::<Vec<_>>()
        .into();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        if ui
            .add_enabled_ui(!uris.is_empty(), |ui| {
                theme::circle_button(
                    ui,
                    Icon::PlayFilled,
                    56.0,
                    palette.accent,
                    palette.accent_hover,
                    palette.on_accent,
                    &gettext(locale, "Play"),
                )
            })
            .inner
            .clicked()
        {
            app.actions.push(Action::PlayUris {
                uris: uris.to_vec(),
                index: 0,
            });
        }
        let shuffle = app.playing_context_shuffle();
        if theme::icon_button(
            ui,
            Icon::Shuffle,
            26.0,
            if shuffle {
                palette.accent
            } else {
                palette.secondary
            },
            if shuffle {
                palette.accent_hover
            } else {
                palette.text
            },
            &if shuffle {
                gettext(locale, "Shuffle off")
            } else {
                gettext(locale, "Shuffle")
            },
        )
        .clicked()
        {
            app.actions.push(Action::SetShuffle(!shuffle));
        }
        theme::text(
            ui,
            songs_and_duration(locale, count, duration),
            theme::regular(13.5),
            palette.secondary,
        );
    });
    ui.add_space(12.0);

    table(
        app,
        ui,
        Table {
            items: &items,
            row_offset: 0,
            pagination: None,
            context: RowContext::Uris(Arc::clone(&uris)),
            show_album: true,
            show_cover: true,
            show_added: false,
            show_added_by: false,
            show_plays: false,
            page: Page::NewReleases,
            loading: false,
            error: None,
            can_load_more: false,
            filter: "",
            items_revision: generation,
            plays_shown_rows: false,
            genre: None,
        },
    );
}

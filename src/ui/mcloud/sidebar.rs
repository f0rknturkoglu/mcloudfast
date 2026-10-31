//! The sidebar after Apple Music on macOS: navigation, the library's
//! shelves and the playlists, each led by an accent-coloured icon or its
//! artwork, with the account in the bottom-left corner.

use egui::{CornerRadius, Frame, Margin, Rect, Sense, UiBuilder, Vec2, pos2, vec2};

use crate::api::models::{Playlist, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon, Palette};

const ROW_HEIGHT: f32 = 32.0;
const ROW_RADIUS: u8 = 7;
const ICON_SIZE: f32 = 18.0;
const ART_SIZE: f32 = 22.0;
/// Where a row's label starts, past its icon or artwork.
const LABEL_LEFT: f32 = 38.0;
const ACCOUNT_HEIGHT: f32 = 48.0;
/// The recently played playlists listed under the pinned ones while the
/// list is short, as mcloud's default.
const RECENT_PLAYLISTS: usize = 5;

/// What leads a row.
enum Leading<'a> {
    Icon(Icon),
    Art(Option<&'a str>, &'a crate::images::ArtLoader),
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    // The traffic lights float over the sidebar's top-left corner.
    let top = 12
        + theme::titlebar_inset(ui.ctx()) as i8
        + super::super::linux_left_controls_inset(ui.ctx()) as i8;
    let beside = if app.show_queue_panel || app.show_lyrics_panel || app.show_friends_panel {
        theme::SIDE_PANEL_MIN_WIDTH
    } else {
        0.0
    };
    // The same panel and width as Spotifast's sidebar, so switching looks
    // keeps the width chosen and the room the page needs.
    let fit = super::super::yielding_panel(
        ui.ctx(),
        "sidebar",
        super::super::SIDEBAR_MIN_WIDTH..=600.0,
        app.settings.sidebar_width,
        ui.available_width() - super::super::topbar::least_width(ui.ctx()) - beside,
    );
    let response = egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(app.settings.sidebar_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin {
            left: 10,
            right: 10,
            top,
            bottom: 8,
        }))
        .show(ui, |ui| {
            let area = ui.max_rect();
            let list = area.with_max_y(area.bottom() - ACCOUNT_HEIGHT);
            ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("mcloud-sidebar")
                    .auto_shrink(false)
                    .show(ui, |ui| contents(app, ui));
            });
            let account = Rect::from_min_max(pos2(area.left(), list.bottom()), area.max);
            ui.scope_builder(UiBuilder::new().max_rect(account), |ui| {
                account_chip(app, ui);
            });
        });
    let width = response.response.rect.width();
    if (width - app.settings.sidebar_width).abs() > 1.0
        && super::super::panel_width_chosen(ui.ctx(), "sidebar", &fit)
    {
        app.settings.sidebar_width = width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn contents(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let page = app.page().clone();
    ui.spacing_mut().item_spacing.y = 2.0;

    if row(
        ui,
        &palette,
        Leading::Icon(Icon::Search),
        &gettext(locale, "Search"),
        page == Page::Search,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::FocusSearch);
    }
    for (icon, label, target) in [
        (Icon::House, gettext(locale, "Home"), Page::Home),
        (
            Icon::Sparkles,
            gettext(locale, "New releases"),
            Page::NewReleases,
        ),
    ] {
        if row(
            ui,
            &palette,
            Leading::Icon(icon),
            &label,
            page == target,
            false,
        )
        .clicked()
        {
            app.actions.push(Action::Open(target));
        }
    }

    heading(ui, &palette, &gettext(locale, "Library"));
    for (icon, label, target) in [
        (Icon::Mic, gettext(locale, "Artists"), Page::Artists),
        (Icon::Disc, gettext(locale, "Albums"), Page::Albums),
        (Icon::Music, gettext(locale, "Songs"), Page::LikedSongs),
        (Icon::Radio, gettext(locale, "Podcasts"), Page::Podcasts),
    ] {
        if row(
            ui,
            &palette,
            Leading::Icon(icon),
            &label,
            page == target,
            false,
        )
        .clicked()
        {
            app.actions.push(Action::Open(target));
        }
    }

    playlists(app, ui, &page);
    ui.add_space(8.0);
}

/// Pinned playlists first, in pin order, then the most recently played,
/// as Spotify's own sidebar sorted by Recents; "Show all" lists the rest.
fn playlists(app: &mut App, ui: &mut egui::Ui, page: &Page) {
    let palette = app.palette;
    let locale = app.locale;
    let expanded_id = egui::Id::new("mcloud-playlists-expanded");
    let mut expanded = ui
        .data(|data| data.get_temp::<bool>(expanded_id))
        .unwrap_or(false);

    let Some(all) = app.library.playlists.get() else {
        heading(ui, &palette, &gettext(locale, "Playlists"));
        return;
    };
    let order = playlist_order(
        all,
        &app.settings.pinned_contexts,
        &app.recent_contexts,
        expanded,
    );
    let more = !expanded && order.len() < all.len();

    let show_less = gettext(locale, "Show less");
    if heading_with_action(
        ui,
        &palette,
        &gettext(locale, "Playlists"),
        expanded.then_some(show_less.as_ref()),
    ) {
        expanded = false;
    }

    let playing = app.playing_context_uri();
    let art_px = app.art_px(ART_SIZE);
    let mut open = None;
    for index in order {
        let playlist = &all[index];
        let image = pick_image(&playlist.images, art_px);
        let active = matches!(page, Page::Playlist(id) if *id == playlist.id);
        let here = playing.as_deref() == Some(playlist.uri.as_str());
        if row(
            ui,
            &palette,
            Leading::Art(image, app.backend.art()),
            &playlist.name,
            active,
            here,
        )
        .clicked()
        {
            open = Some(Page::Playlist(playlist.id.clone()));
        }
    }
    if more
        && row(
            ui,
            &palette,
            Leading::Icon(Icon::LayoutGrid),
            &gettext(locale, "Show all"),
            false,
            false,
        )
        .clicked()
    {
        expanded = true;
    }
    if let Some(page) = open {
        app.actions.push(Action::Open(page));
    }
    ui.data_mut(|data| data.insert_temp(expanded_id, expanded));
}

/// The indices of the playlists to list, in order: pinned ones, then the
/// rest by Recents, as Spotify sorts them: the recently played first, then
/// the others in library order. Unless `all` is asked for, only
/// [`RECENT_PLAYLISTS`] follow the pinned ones.
fn playlist_order(
    playlists: &[Playlist],
    pinned: &[String],
    recent: &[String],
    all: bool,
) -> Vec<usize> {
    let position = |uri: &str| playlists.iter().position(|playlist| playlist.uri == uri);
    let mut order: Vec<usize> = Vec::new();
    for index in pinned.iter().filter_map(|uri| position(uri)) {
        if !order.contains(&index) {
            order.push(index);
        }
    }
    let shown = if all {
        playlists.len()
    } else {
        (order.len() + RECENT_PLAYLISTS).min(playlists.len())
    };
    let by_recents = recent
        .iter()
        .filter_map(|uri| position(uri))
        .chain(0..playlists.len());
    for index in by_recents {
        if order.len() == shown {
            break;
        }
        if !order.contains(&index) {
            order.push(index);
        }
    }
    order
}

fn heading(ui: &mut egui::Ui, palette: &Palette, label: &str) {
    heading_with_action(ui, palette, label, None);
}

/// A section heading, with a quiet text button on its right when `action`
/// is given. Returns whether that button was clicked.
fn heading_with_action(
    ui: &mut egui::Ui,
    palette: &Palette,
    label: &str,
    action: Option<&str>,
) -> bool {
    ui.add_space(14.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    ui.painter().text(
        pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::semibold(11.5),
        palette.dim,
    );
    let Some(action) = action else {
        return false;
    };
    let galley =
        ui.painter()
            .layout_no_wrap(action.to_owned(), theme::medium(11.5), palette.accent);
    let button = Rect::from_min_size(
        pos2(rect.right() - galley.size().x - 8.0, rect.top()),
        vec2(galley.size().x + 8.0, rect.height()),
    );
    let response = ui.interact(
        button,
        ui.id().with(("heading-action", label)),
        Sense::click(),
    );
    let color = if response.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    ui.painter().galley(
        pos2(button.left(), button.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, action));
    response.clicked()
}

/// A navigation or playlist row. `playing` marks the playlist that is
/// playing with a speaker at the row's end.
fn row(
    ui: &mut egui::Ui,
    palette: &Palette,
    leading: Leading<'_>,
    label: &str,
    active: bool,
    playing: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if active {
            palette.surface_active
        } else if response.hovered() {
            palette.surface_hover
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
        let lead = Rect::from_center_size(
            pos2(rect.left() + 8.0 + ART_SIZE / 2.0, rect.center().y),
            Vec2::splat(ART_SIZE),
        );
        match leading {
            Leading::Icon(icon) => {
                let icon_rect = Rect::from_center_size(lead.center(), Vec2::splat(ICON_SIZE));
                icon.image(palette.accent, ICON_SIZE)
                    .paint_at(ui, icon_rect);
            }
            Leading::Art(url, art) => {
                super::super::widgets::paint_cover(
                    ui,
                    palette,
                    url,
                    lead,
                    4.0,
                    Icon::Music,
                    Some(art),
                );
            }
        }
        let trailing = if playing { 24.0 } else { 8.0 };
        let galley = super::super::widgets::ellipsized(
            ui,
            label,
            theme::medium(14.0),
            palette.text,
            rect.width() - LABEL_LEFT - trailing,
            1,
        );
        ui.painter().galley(
            pos2(
                rect.left() + LABEL_LEFT,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            palette.text,
        );
        if playing {
            let speaker = Rect::from_center_size(
                pos2(rect.right() - 14.0, rect.center().y),
                Vec2::splat(14.0),
            );
            Icon::Volume2
                .image(palette.accent, 14.0)
                .paint_at(ui, speaker);
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, label)
    });
    theme::focus_ring(ui, &response);
    response
}

/// The account in the bottom-left corner, as in Apple Music. It opens
/// Settings, where the account card is.
fn account_chip(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let (name, avatar) = app
        .user
        .as_ref()
        .map(|user| {
            (
                user.name().to_string(),
                pick_image(&user.images, app.art_px(28.0)).map(str::to_string),
            )
        })
        .unwrap_or_default();
    let area = ui.max_rect();
    let rect = Rect::from_min_max(pos2(area.left(), area.bottom() - 40.0), area.max);
    let response = ui.interact(rect, ui.id().with("mcloud-account"), Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().hline(
            area.x_range(),
            rect.top() - 4.0,
            egui::Stroke::new(1.0, palette.outline),
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(ROW_RADIUS), palette.surface_hover);
        }
        let face = Rect::from_center_size(
            pos2(rect.left() + 8.0 + 14.0, rect.center().y),
            Vec2::splat(28.0),
        );
        match avatar.as_deref() {
            Some(url) => super::super::widgets::paint_cover(
                ui,
                &palette,
                Some(url),
                face,
                14.0,
                Icon::User,
                Some(app.backend.art()),
            ),
            None => {
                let initial = name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                ui.painter()
                    .circle_filled(face.center(), 14.0, palette.accent);
                ui.painter().text(
                    face.center(),
                    egui::Align2::CENTER_CENTER,
                    initial,
                    theme::bold(13.0),
                    palette.on_accent,
                );
            }
        }
        let galley = super::super::widgets::ellipsized(
            ui,
            &name,
            theme::semibold(13.0),
            palette.text,
            rect.width() - 46.0 - 8.0,
            1,
        );
        ui.painter().galley(
            pos2(face.right() + 10.0, rect.center().y - galley.size().y / 2.0),
            galley,
            palette.text,
        );
    }
    if response.clicked() {
        app.actions.push(Action::Open(Page::Settings));
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &name)
    });
    theme::focus_ring(ui, &response);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist(uri: &str) -> Playlist {
        serde_json::from_value(serde_json::json!({
            "id": uri.rsplit(':').next().unwrap(),
            "name": uri,
            "uri": uri,
        }))
        .unwrap()
    }

    #[test]
    fn pinned_lead_then_recents_up_to_the_limit_then_the_rest_when_asked() {
        let playlists: Vec<Playlist> = (0..10)
            .map(|n| playlist(&format!("spotify:playlist:p{n}")))
            .collect();
        let uri = |n: usize| format!("spotify:playlist:p{n}");
        let pinned = vec![uri(7), "spotify:album:gone".into()];
        // The pinned one played recently too; it is not listed twice.
        let recent: Vec<String> = [7, 2, 4, 9, 1, 3, 5, 8].map(uri).to_vec();

        assert_eq!(
            playlist_order(&playlists, &pinned, &recent, false),
            vec![7, 2, 4, 9, 1, 3]
        );
        assert_eq!(
            playlist_order(&playlists, &pinned, &recent, true),
            vec![7, 2, 4, 9, 1, 3, 5, 8, 0, 6]
        );
        // Nothing played yet: the library's own order fills the list.
        assert_eq!(
            playlist_order(&playlists, &pinned, &[], false),
            vec![7, 0, 1, 2, 3, 4]
        );
        assert_eq!(playlist_order(&playlists[..2], &[], &[], false), vec![0, 1]);
    }
}

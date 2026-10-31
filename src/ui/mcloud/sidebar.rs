//! The sidebar as mcloud draws it (user.css `#am-sidebar`), after Apple
//! Music on macOS 26: a dark material over the page that lets about a
//! quarter of it through, navigation and the library's shelves led by
//! accent-coloured icons, the playlists with their artwork under a heading
//! with a "+", and the account with friends in the bottom-left corner.

use egui::{
    Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, UiBuilder, Vec2, pos2, vec2,
};

use crate::api::models::{Playlist, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog, Page};
use crate::theme::{self, Icon};

const ROW_HEIGHT: f32 = 32.0;
const ROW_RADIUS: u8 = 8;
const ROW_PADDING: f32 = 16.0;
const ICON_SIZE: f32 = 18.0;
const ART_SIZE: f32 = 22.0;
/// The gap between a row's icon or artwork and its name.
const ICON_GAP: f32 = 10.0;
/// The first row's top, level with the page under the toolbar.
const TOP: f32 = 76.0;
const SIDE_PADDING: f32 = 10.0;
const ACCOUNT_HEIGHT: f32 = 54.0;
/// The recently played playlists listed under the pinned ones while the
/// list is short, as mcloud's default.
const RECENT_PLAYLISTS: usize = 5;

const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
const HEADING: Color32 = Color32::from_rgba_premultiplied(92, 92, 92, 92);
const QUIET: Color32 = Color32::from_rgba_premultiplied(140, 140, 140, 140);

/// What leads a row.
enum Leading<'a> {
    Icon(Icon),
    Art(Option<&'a str>, &'a crate::images::ArtLoader),
    /// "Show all": a small chevron in the icon column.
    Chevron,
}

/// The sidebar's material: #252525 at 76% over the page beside it, so a
/// coloured page shows through as a deep version of itself.
fn material(under: Color32) -> Color32 {
    under.lerp_to_gamma(Color32::from_rgb(0x25, 0x25, 0x25), 0.76)
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let under = super::super::page_tint(app).unwrap_or(app.palette.window);
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
        .frame(Frame::new().fill(material(under)).inner_margin(Margin {
            left: SIDE_PADDING as i8,
            right: SIDE_PADDING as i8,
            top: TOP as i8,
            bottom: 12,
        }))
        .show(ui, |ui| {
            let area = ui.max_rect();
            // A hairline along the right edge, as macOS draws it.
            ui.painter().vline(
                area.right() + SIDE_PADDING - 0.5,
                (area.top() - TOP)..=(area.bottom() + 12.0),
                Stroke::new(1.0, Color32::from_white_alpha(10)),
            );
            let list = area.with_max_y(area.bottom() - ACCOUNT_HEIGHT);
            ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("mcloud-sidebar")
                    .auto_shrink(false)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| contents(app, ui));
            });
            let account = Rect::from_min_max(pos2(area.left(), list.bottom()), area.max);
            ui.scope_builder(UiBuilder::new().max_rect(account), |ui| {
                account_row(app, ui);
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
    let locale = app.locale;
    let page = app.page().clone();
    ui.spacing_mut().item_spacing.y = 0.0;

    if row(
        ui,
        Leading::Icon(Icon::Search),
        &gettext(locale, "Search"),
        page == Page::Search,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::FocusSearch);
    }
    if row(
        ui,
        Leading::Icon(Icon::McloudHome),
        &gettext(locale, "Home"),
        page == Page::Home,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Home));
    }

    heading(ui, &gettext(locale, "Library"), None);
    for (icon, label, target) in [
        (
            Icon::McloudArtist,
            gettext(locale, "Artists"),
            Page::Artists,
        ),
        (Icon::McloudAlbum, gettext(locale, "Albums"), Page::Albums),
        (
            Icon::McloudMusic,
            gettext(locale, "Songs"),
            Page::LikedSongs,
        ),
    ] {
        if row(ui, Leading::Icon(icon), &label, page == target, false).clicked() {
            app.actions.push(Action::Open(target));
        }
    }

    playlists(app, ui, &page);
    ui.add_space(20.0);
}

/// Pinned playlists first, in pin order, then the most recently played,
/// as Spotify's own sidebar sorted by Recents; "Show all" lists the rest.
fn playlists(app: &mut App, ui: &mut egui::Ui, page: &Page) {
    let locale = app.locale;
    let expanded_id = egui::Id::new("mcloud-playlists-expanded");
    let mut expanded = ui
        .data(|data| data.get_temp::<bool>(expanded_id))
        .unwrap_or(false);

    let add = gettext(locale, "Create a playlist");
    let show_less = gettext(locale, "Show less");
    let (less, create) = heading(
        ui,
        &gettext(locale, "Playlists"),
        Some((expanded.then_some(show_less.as_ref()), add.as_ref())),
    );
    if less {
        expanded = false;
    }
    if create {
        app.actions.push(Action::ShowDialog(Dialog::CreatePlaylist {
            name: String::new(),
            public: false,
            add_uris: Vec::new(),
        }));
    }

    let Some(all) = app.library.playlists.get() else {
        return;
    };
    let order = playlist_order(
        all,
        &app.settings.pinned_contexts,
        &app.recent_contexts,
        expanded,
    );
    let more = !expanded && order.len() < all.len();

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
            Leading::Chevron,
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

/// A section heading. With `buttons`, it carries "Show less" (when given)
/// and a "+" on its right; returns whether each was clicked.
fn heading(ui: &mut egui::Ui, label: &str, buttons: Option<(Option<&str>, &str)>) -> (bool, bool) {
    ui.add_space(16.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 13.0), Sense::hover());
    ui.add_space(4.0);
    ui.painter().text(
        pos2(rect.left() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::semibold(11.0),
        HEADING,
    );
    let Some((less, add)) = buttons else {
        return (false, false);
    };
    // The "+" sits over the pin column, 5 points from the edge.
    let plus = Rect::from_center_size(
        pos2(rect.right() - 5.0 - 11.0, rect.center().y + 1.0),
        Vec2::splat(22.0),
    );
    let plus_response = ui.interact(plus, ui.id().with(("heading-add", label)), Sense::click());
    plus_response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, add));
    if plus_response.hovered() {
        ui.painter()
            .rect_filled(plus, CornerRadius::same(6), Color32::from_white_alpha(20));
    }
    theme::paint_icon(
        ui,
        Icon::McloudPlus,
        plus,
        15.0,
        if plus_response.hovered() { TEXT } else { QUIET },
    );
    let mut less_clicked = false;
    if let Some(less) = less {
        let galley = ui
            .painter()
            .layout_no_wrap(less.to_owned(), theme::semibold(11.0), QUIET);
        let pill = Rect::from_min_size(
            pos2(
                plus.left() - 4.0 - galley.size().x - 16.0,
                rect.center().y - 10.0,
            ),
            vec2(galley.size().x + 16.0, 20.0),
        );
        let response = ui.interact(pill, ui.id().with(("heading-less", label)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, less));
        let (fill, color) = if response.hovered() {
            (Color32::from_white_alpha(31), TEXT)
        } else {
            (Color32::from_white_alpha(15), QUIET)
        };
        ui.painter().rect_filled(pill, CornerRadius::same(10), fill);
        ui.painter().galley(
            pos2(pill.left() + 8.0, pill.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        less_clicked = response.clicked();
    }
    (less_clicked, plus_response.on_hover_text(add).clicked())
}

/// A navigation or playlist row. `playing` marks the playlist that is
/// playing with a speaker at the row's end.
fn row(
    ui: &mut egui::Ui,
    leading: Leading<'_>,
    label: &str,
    active: bool,
    playing: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if active {
            Color32::from_white_alpha(31)
        } else if response.hovered() {
            Color32::from_white_alpha(13)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
        // Playlist rows sit two points further left, so their artwork's
        // centre lines up with the icons above.
        let (lead_left, lead_size) = match leading {
            Leading::Art(..) => (rect.left() + 14.0, ART_SIZE),
            _ => (rect.left() + ROW_PADDING, ICON_SIZE),
        };
        let lead = Rect::from_min_size(
            pos2(lead_left, rect.center().y - lead_size / 2.0),
            Vec2::splat(lead_size),
        );
        let mut color = TEXT;
        match leading {
            Leading::Icon(icon) => {
                icon.image(accent(), ICON_SIZE).paint_at(ui, lead);
            }
            Leading::Art(url, art) => {
                ui.painter().rect_filled(
                    lead,
                    CornerRadius::same(4),
                    Color32::from_white_alpha(20),
                );
                crate::ui::widgets::paint_cover(
                    ui,
                    &crate::theme::Palette::mcloud(),
                    url,
                    lead,
                    4.0,
                    Icon::McloudMusic,
                    Some(art),
                );
            }
            Leading::Chevron => {
                color = QUIET;
                theme::paint_icon(ui, Icon::McloudChevron, lead, 12.0, QUIET);
            }
        }
        let font = if active {
            theme::semibold(13.0)
        } else {
            theme::medium(13.0)
        };
        let gap = if matches!(leading, Leading::Art(..)) {
            8.0
        } else {
            ICON_GAP
        };
        let text_left = lead.right() + gap;
        let trailing = if playing {
            14.0 + 8.0 + ROW_PADDING
        } else {
            ROW_PADDING
        };
        let galley = crate::ui::widgets::ellipsized(
            ui,
            label,
            font,
            color,
            (rect.right() - trailing - text_left).max(0.0),
            1,
        );
        ui.painter().galley(
            pos2(text_left, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        if playing {
            let speaker = Rect::from_center_size(
                pos2(rect.right() - ROW_PADDING - 7.0, rect.center().y),
                Vec2::splat(14.0),
            );
            theme::paint_icon(ui, Icon::McloudVolume2, speaker, 14.0, accent());
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, label)
    });
    theme::focus_ring(ui, &response);
    response
}

/// mcloud's accent: its palette is fixed.
fn accent() -> Color32 {
    crate::theme::Palette::mcloud().accent
}

/// The account in the bottom-left corner, as in Apple Music: the chip opens
/// Settings, where the account card is, and the button beside it shows
/// what friends are listening to.
fn account_row(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
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
    let line = Rect::from_min_max(pos2(area.left(), area.bottom() - 40.0), area.max);
    let friends = Rect::from_center_size(
        pos2(line.right() - 15.0, line.center().y),
        Vec2::splat(30.0),
    );
    let chip = Rect::from_min_max(line.min, pos2(friends.left() - 2.0, line.bottom()));

    let response = ui.interact(chip, ui.id().with("mcloud-account"), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(chip, CornerRadius::same(8), Color32::from_white_alpha(13));
    }
    let face = Rect::from_center_size(
        pos2(chip.left() + 8.0 + 14.0, chip.center().y),
        Vec2::splat(28.0),
    );
    match avatar.as_deref() {
        Some(url) => crate::ui::widgets::paint_cover(
            ui,
            &palette,
            Some(url),
            face,
            14.0,
            Icon::User,
            Some(app.backend.art()),
        ),
        None => {
            crate::ui::widgets::paint_vertical_gradient(
                ui,
                face,
                Color32::from_rgb(0xa6, 0xb4, 0xd6),
                Color32::from_rgb(0x6d, 0x7b, 0xa0),
            );
            let initial = name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            ui.painter().text(
                face.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                theme::semibold(11.0),
                Color32::WHITE,
            );
        }
    }
    let galley = crate::ui::widgets::ellipsized(
        ui,
        &name,
        theme::semibold(13.0),
        TEXT,
        (chip.right() - 8.0 - face.right() - 10.0).max(0.0),
        1,
    );
    ui.painter().galley(
        pos2(face.right() + 10.0, chip.center().y - galley.size().y / 2.0),
        galley,
        TEXT,
    );
    if response.clicked() {
        app.actions.push(Action::Open(Page::Settings));
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &name)
    });
    theme::focus_ring(ui, &response);

    let label = gettext(locale, "Listening activity");
    let response = ui.interact(friends, ui.id().with("mcloud-friends"), Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    let color = if app.show_friends_panel {
        accent()
    } else if response.hovered() {
        TEXT
    } else {
        Color32::from_rgb(0xa9, 0xa9, 0xa9)
    };
    if response.hovered() {
        ui.painter().rect_filled(
            friends,
            CornerRadius::same(8),
            Color32::from_white_alpha(13),
        );
    }
    theme::paint_icon(ui, Icon::McloudFriends, friends, 18.0, color);
    if response.on_hover_text(label.as_ref()).clicked() {
        app.actions.push(Action::ToggleFriendsPanel);
    }
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

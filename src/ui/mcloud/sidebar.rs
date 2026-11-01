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

/// The edge glow along the right side of the sidebar, matching mcloud's `#am-sidebar-glow`
/// (user.css line 206): a heavily blurred, saturated and brightened backdrop reaching
/// 160px inward from the sidebar edge, with a soft compression roll-off to avoid blow-out.
fn glow_color(tint: Color32) -> Option<Color32> {
    let r = tint.r() as f32 / 255.0;
    let g = tint.g() as f32 / 255.0;
    let b = tint.b() as f32 / 255.0;

    let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    if max_c - min_c < 0.04 && l < 0.25 {
        return None;
    }

    // Boost saturation (3.5x as in mcloud CSS backdrop-filter: saturate(3.5))
    let mut sr = l + (r - l) * 3.5;
    let mut sg = l + (g - l) * 3.5;
    let mut sb = l + (b - l) * 3.5;

    // Brightness (1.25x as in mcloud CSS backdrop-filter: brightness(1.25))
    sr *= 1.25;
    sg *= 1.25;
    sb *= 1.25;

    // Soft cap roll-off above 0.4 up to ~0.66 (matching #am-glow-cap tableValues)
    let cap = |v: f32| -> f32 {
        let v = v.clamp(0.0, 3.0);
        if v <= 0.4 {
            v
        } else {
            0.4 + (v - 0.4).atan() * (0.26 / (std::f32::consts::PI / 2.0))
        }
    };

    let cr = (cap(sr).clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (cap(sg).clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (cap(sb).clamp(0.0, 1.0) * 255.0).round() as u8;

    Some(Color32::from_rgb(cr, cg, cb))
}

fn paint_sidebar_glow(
    painter: &egui::Painter,
    edge_x: f32,
    top_y: f32,
    bottom_y: f32,
    tint: Color32,
) {
    let Some(glow_rgb) = glow_color(tint) else {
        return;
    };

    let mut mesh = egui::Mesh::default();
    // 160px inward matching linear-gradient(90deg, transparent calc(var(--am-side) - 160px), #0007 calc(var(--am-side) - 70px), #000 calc(var(--am-side) - 1px), transparent var(--am-side))
    let xs = [edge_x - 160.0, edge_x - 70.0, edge_x - 1.0, edge_x];
    let x_mult = [0.0, 0.44, 1.0, 0.0];
    let ys = [top_y, top_y + 160.0, top_y + 320.0, top_y + 540.0, bottom_y];
    let y_mult = [1.0, 1.0, 0.75, 0.30, 0.05];

    let r = glow_rgb.r();
    let g = glow_rgb.g();
    let b = glow_rgb.b();
    const PEAK_ALPHA: f32 = 85.0;

    for (iy, &y) in ys.iter().enumerate() {
        for (ix, &x) in xs.iter().enumerate() {
            let a = (PEAK_ALPHA * x_mult[ix] * y_mult[iy]).clamp(0.0, 255.0) as u8;
            let col = Color32::from_rgba_unmultiplied(r, g, b, a);
            mesh.colored_vertex(pos2(x, y), col);
        }
    }

    let cols = xs.len() as u32;
    for iy in 0..(ys.len() - 1) {
        let row_start = iy as u32 * cols;
        let next_row = row_start + cols;
        for ix in 0..(xs.len() - 1) {
            let tl = row_start + ix as u32;
            let tr = tl + 1;
            let bl = next_row + ix as u32;
            let br = bl + 1;
            mesh.add_triangle(tl, tr, br);
            mesh.add_triangle(tl, br, bl);
        }
    }

    painter.add(egui::Shape::mesh(mesh));
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let tint = super::super::page_tint(app).or_else(|| app.now_playing_tint());
    let under = tint.unwrap_or(app.palette.window);
    // The artist page tints the whole window (mcloud's `am-tinted`): the
    // sidebar is its colour at 34% in #161616, 85% over the page, and has
    // no glow.
    let page = app.page().clone();
    let (fill, tint) = match &page {
        Page::Artist(id) => {
            let (top, _) = super::artist::page_colours(app, id);
            let mixed = top.lerp_to_gamma(Color32::from_rgb(0x16, 0x16, 0x16), 0.66);
            (mixed.lerp_to_gamma(top, 0.15), None)
        }
        _ => (material(under), tint),
    };
    let beside = if app.show_queue_panel || app.show_lyrics_panel || app.show_friends_panel {
        theme::SIDE_PANEL_MIN_WIDTH
    } else {
        0.0
    };
    // Spotifast's sidebar panel, at mcloud's width: the one dragged to,
    // else 15.5% of the window, 220 to 300pt (user.css `--am-side`).
    let chosen = app
        .settings
        .mcloud_sidebar_width
        .unwrap_or_else(|| default_width(ui.ctx().content_rect().width()));
    let fit = super::super::yielding_panel(
        ui.ctx(),
        "sidebar",
        super::super::SIDEBAR_MIN_WIDTH..=600.0,
        chosen,
        ui.available_width() - super::super::topbar::least_width(ui.ctx()) - beside,
    );
    let response = egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(chosen)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(Frame::new().fill(fill).inner_margin(Margin {
            left: SIDE_PADDING as i8,
            right: SIDE_PADDING as i8,
            top: TOP as i8,
            bottom: 12,
        }))
        .show(ui, |ui| {
            let area = ui.max_rect();
            let edge_x = area.right() + SIDE_PADDING;
            let top_y = area.top() - TOP;
            let bottom_y = area.bottom() + 12.0;

            if let Some(t) = tint {
                paint_sidebar_glow(ui.painter(), edge_x, top_y, bottom_y, t);
            }

            // A hairline along the right edge, as macOS draws it.
            ui.painter().vline(
                edge_x - 0.5,
                top_y..=bottom_y,
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
    // Only a drag sets a width of its own: the default follows the window.
    let width = response.response.rect.width();
    if (width - chosen).abs() > 1.0 && super::super::panel_dragged(ui.ctx(), "sidebar") {
        app.settings.mcloud_sidebar_width = Some(width);
        app.actions.push(Action::SettingsChanged);
    }
}

/// mcloud's sidebar width for a window: `clamp(220px, 15.5vw, 300px)`.
fn default_width(window: f32) -> f32 {
    (window * 0.155).clamp(220.0, 300.0)
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
    if row(
        ui,
        Leading::Icon(Icon::McloudBrowse),
        &gettext(locale, "Browse"),
        page == Page::Browse,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Browse));
    }

    heading(ui, &gettext(locale, "Library"), None);
    for (icon, label, target) in [
        (
            Icon::McloudClock,
            gettext(locale, "Recently Added"),
            Page::RecentlyAdded,
        ),
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

    let all_playlists = gettext(locale, "All Playlists");
    if row(
        ui,
        Leading::Icon(Icon::McloudGrid),
        &all_playlists,
        *page == Page::AllPlaylists,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::AllPlaylists));
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

    /// Like mcloud's, the sidebar is 15.5% of the window, 220 to 300pt.
    #[test]
    fn the_sidebar_follows_the_window_between_220_and_300() {
        assert_eq!(default_width(1000.0), 220.0);
        assert!((default_width(1600.0) - 248.0).abs() < 0.01);
        assert_eq!(default_width(2560.0), 300.0);
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

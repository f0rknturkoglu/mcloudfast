//! Now Playing beside the player, as mcloud's `#am-nowplaying`: a glass card
//! over the page or a pane docked at the window's edge, with the artwork, the
//! title and artist, the save star and "…", and the lyrics under them.

use egui::{Color32, CornerRadius, Id, Rect, Sense, UiBuilder, Vec2, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::settings::{McloudNpBackground, McloudNpLayout};
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets;

const MARGIN: f32 = 16.0;
/// What the card leaves under it for the floating player.
const BOTTOM: f32 = 96.0;
const PADDING: f32 = 22.0;
const GAP: f32 = 14.0;
const META: f32 = 50.0;
const ACTIONS: f32 = 36.0;
const CHIP: f32 = 30.0;
const MIN_ART: f32 = 110.0;
/// Under this the lyrics give up room rather than the artwork vanish.
const MIN_LYRICS: f32 = 120.0;

/// mcloud's `--am-np-w`: 24% of the window, 300 to 380pt.
fn pane_width(window: f32) -> f32 {
    (window * 0.24).clamp(300.0, 380.0)
}

/// The artwork's side: the pane's width, less what the lyrics need.
fn art_side(width: f32, height: f32, lyrics: bool) -> f32 {
    let widest = width - 2.0 * PADDING;
    let used = PADDING + GAP + META + GAP + ACTIONS + 14.0;
    let room = height - used - if lyrics { GAP + MIN_LYRICS } else { 0.0 };
    room.min(widest).max(MIN_ART)
}

/// mcloud's slide, `cubic-bezier(.32,.72,0,1)`, approximated by an ease-out.
fn slide(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(4)
}

pub(in crate::ui) fn docked(app: &App) -> bool {
    app.settings.mcloud_np_layout == McloudNpLayout::Docked
}

/// The docked pane, full height at the window's edge.
pub(in crate::ui) fn docked_panel(app: &mut App, ui: &mut egui::Ui) {
    let width = pane_width(ui.ctx().content_rect().width());
    egui::Panel::right("mcloud-now-playing")
        .exact_size(width)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let rect = ui.max_rect();
            ui.painter().vline(
                rect.left(),
                rect.y_range(),
                egui::Stroke::new(1.0, app.palette.text.gamma_multiply(0.078)),
            );
            body(app, ui, rect, 0.0);
        });
}

/// The floating card over `content`, sliding in and out at the right edge.
pub(in crate::ui) fn card(app: &mut App, ui: &mut egui::Ui, content: Rect) {
    if docked(app) || app.lyrics_fullscreen.is_some() {
        return;
    }
    let ctx = ui.ctx().clone();
    let shown = app.show_lyrics_panel;
    let reveal = if crate::motion::reduced(&ctx) {
        f32::from(u8::from(shown))
    } else {
        ctx.animate_bool_with_time(Id::new("mcloud-now-playing-reveal"), shown, 0.24)
    };
    if reveal <= 0.0 {
        return;
    }
    let width = pane_width(ctx.content_rect().width());
    let top = content.top() + super::topbar::HEIGHT;
    let travel = (width + MARGIN) * (1.0 - slide(reveal));
    let rect = Rect::from_min_max(
        pos2(content.right() - MARGIN - width + travel, top),
        pos2(content.right() - MARGIN + travel, content.bottom() - BOTTOM),
    );
    if rect.height() < MIN_ART + META {
        return;
    }
    egui::Area::new(Id::new("mcloud-now-playing"))
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .constrain(false)
        .show(&ctx, |ui| {
            ui.set_clip_rect(rect.expand(40.0).intersect(ctx.content_rect()));
            // Clicks on the card stay on it rather than reaching the page.
            ui.allocate_exact_size(rect.size(), Sense::click());
            ui.painter().add(
                egui::epaint::Shadow {
                    offset: [0, 18],
                    blur: 50,
                    spread: 0,
                    color: Color32::from_black_alpha(119),
                }
                .as_shape(rect, CornerRadius::same(18)),
            );
            body(app, ui, rect, 18.0);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(18),
                egui::Stroke::new(1.0, Color32::from_white_alpha(18)),
                egui::StrokeKind::Inside,
            );
        });
}

/// The colours over the artwork: light whatever the app's appearance, as the
/// picture behind them is dark.
fn over_artwork(palette: &Palette) -> Palette {
    let mut over = Palette::mcloud_for_darkness(true);
    over.accent = palette.accent;
    over.accent_hover = palette.accent_hover;
    over
}

fn body(app: &mut App, ui: &mut egui::Ui, rect: Rect, radius: f32) {
    let background = app.settings.mcloud_np_background;
    let plain = background == McloudNpBackground::Plain;
    let saved_palette = app.palette;
    paint_background(app, ui, rect, radius, background);
    if background == McloudNpBackground::Artwork {
        app.palette = over_artwork(&saved_palette);
    }
    let palette = app.palette;
    let lyrics_shown = plain || app.settings.mcloud_np_lyrics;

    let mut y = rect.top() + PADDING;
    let left = rect.left() + PADDING;
    let right = rect.right() - PADDING;
    let now = app.now_playing();
    if let Some(now) = now.as_ref().filter(|_| !plain) {
        let side = art_side(rect.width(), rect.height(), lyrics_shown);
        let art = Rect::from_min_size(pos2(rect.center().x - side / 2.0, y), Vec2::splat(side));
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 14],
                blur: 40,
                spread: 0,
                color: Color32::from_black_alpha(140),
            }
            .as_shape(art, CornerRadius::same(10)),
        );
        widgets::paint_cover(
            ui,
            &palette,
            now.art_url.as_deref(),
            art,
            10.0,
            Icon::Music,
            Some(app.backend.art()),
        );
        let label = gettext(app.locale, "Full screen lyrics");
        let response = ui
            .interact(art, Id::new("mcloud-np-art"), Sense::click())
            .on_hover_text(label.as_ref());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        theme::focus_ring(ui, &response);
        if response.clicked() {
            app.actions.push(Action::SetLyricsFullscreen(true));
        }
        y = art.bottom() + GAP;
        let meta = Rect::from_min_max(pos2(left, y), pos2(right, y + META));
        meta_row(app, ui, meta, now);
        y = meta.bottom() + GAP;
    }

    let bottom = rect.bottom() - 14.0 - ACTIONS;
    if lyrics_shown && bottom > y {
        // A line not being sung is laid out 1/0.95 wider than its column and
        // drawn smaller, so the column is narrowed by as much.
        let area = Rect::from_min_max(pos2(left, y), pos2(left + (right - left) * 0.95, bottom));
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        child.set_clip_rect(area.expand2(vec2(PADDING, 0.0)).intersect(ui.clip_rect()));
        crate::ui::lyrics::contents(app, &mut child);
    }

    actions(app, ui, rect);
    close_button(app, ui, rect);
    app.palette = saved_palette;
}

fn paint_background(
    app: &mut App,
    ui: &egui::Ui,
    rect: Rect,
    radius: f32,
    background: McloudNpBackground,
) {
    let round = CornerRadius::same(radius as u8);
    let palette = app.palette;
    match background {
        McloudNpBackground::Plain => {
            let fill = if palette.dark {
                Color32::from_gray(0x2c)
            } else {
                Color32::from_rgb(0xee, 0xed, 0xec)
            };
            ui.painter().rect_filled(rect, round, fill);
        }
        McloudNpBackground::Glass => {
            if !super::glass::paint_menu(ui, rect, app.settings.mcloud_glass) {
                ui.painter().rect_filled(rect, round, palette.panel);
            }
        }
        McloudNpBackground::Artwork => {
            ui.painter()
                .rect_filled(rect, round, Color32::from_gray(0x1b));
            let art_loader = app.backend.art().clone();
            let cover = app
                .now_playing()
                .and_then(|now| now.art_small.or(now.art_url));
            if let Some(texture) =
                app.lyrics_backdrop
                    .texture(ui.ctx(), &art_loader, cover.as_deref())
            {
                // The middle of the blurred cover, stretched and darkened.
                let uv = Rect::from_min_max(pos2(0.17, 0.17), pos2(0.83, 0.83));
                egui::Image::new(texture)
                    .uv(uv)
                    .tint(Color32::from_gray(158))
                    .corner_radius(round)
                    .paint_at(ui, rect);
            }
            ui.painter()
                .rect_filled(rect, round, Color32::from_black_alpha(72));
        }
    }
}

/// The title and artist, each opening its page, the save star and "…".
fn meta_row(app: &mut App, ui: &mut egui::Ui, meta: Rect, now: &crate::app::NowPlaying) {
    let palette = app.palette;
    let locale = app.locale;
    let sub = palette.text.gamma_multiply(0.65);
    let chips = 2.0 * CHIP + 10.0;
    let text_width = (meta.width() - chips - 10.0).max(20.0);
    let title = widgets::ellipsized(
        ui,
        &now.title,
        theme::semibold(19.0),
        palette.text,
        text_width,
        1,
    );
    let artist = widgets::ellipsized(ui, &now.subtitle, theme::regular(17.0), sub, text_width, 1);
    let total = title.size().y + 1.0 + artist.size().y;
    let top = meta.center().y - total / 2.0;
    let title_rect = Rect::from_min_size(pos2(meta.left(), top), title.size());
    let artist_rect =
        Rect::from_min_size(pos2(meta.left(), title_rect.bottom() + 1.0), artist.size());
    ui.painter().galley(title_rect.min, title, palette.text);
    ui.painter().galley(artist_rect.min, artist, sub);
    let title_response = ui
        .interact(title_rect, Id::new("mcloud-np-title"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if title_response.clicked() {
        if let Some(id) = &now.album_id {
            app.actions.push(Action::Open(Page::Album(id.clone())));
        } else if let Some(id) = &now.show_id {
            app.actions.push(Action::Open(Page::Show(id.clone())));
        }
    }
    let artist_response = ui
        .interact(artist_rect, Id::new("mcloud-np-artist"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if artist_response.clicked()
        && let Some(id) = now.artists.iter().find_map(|artist| artist.id.clone())
    {
        app.actions.push(Action::Open(Page::Artist(id)));
    }

    let chip = palette.text.gamma_multiply(0.12);
    let more = Rect::from_center_size(
        pos2(meta.right() - CHIP / 2.0, meta.center().y),
        Vec2::splat(CHIP),
    );
    let star = more.translate(vec2(-CHIP - 10.0, 0.0));
    if !now.is_episode {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        let label = gettext(
            locale,
            if saved {
                "Remove from Liked Songs"
            } else {
                "Save to Liked Songs"
            },
        );
        let response = ui.interact(star, Id::new("mcloud-np-star"), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        ui.painter().circle_filled(star.center(), CHIP / 2.0, chip);
        let (icon, color) = if saved {
            (Icon::McloudStarFilled, palette.accent)
        } else {
            (Icon::McloudStar, palette.text)
        };
        theme::paint_icon(ui, icon, star, 17.0, color);
        theme::focus_ring(ui, &response);
        if response.on_hover_text(label.as_ref()).clicked() {
            app.actions.push(Action::ToggleSaved(now.uri.clone()));
        }
    }
    if let Some(item) = app.now_playing_item() {
        let label = gettext(locale, "More options");
        let response = ui.interact(more, Id::new("mcloud-np-more"), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        ui.painter().circle_filled(more.center(), CHIP / 2.0, chip);
        theme::paint_icon(ui, Icon::McloudMore, more, 17.0, palette.text);
        theme::focus_ring(ui, &response);
        let context = app.editable_context_playlist();
        egui::Popup::menu(&response)
            .frame(widgets::menu_frame(&palette))
            .align(egui::RectAlign::TOP_END)
            .show(|ui| widgets::item_menu(ui, app, &item, context.as_ref(), None));
    }
}

/// The bottom row: full-screen lyrics, and Follow when the lyrics were scrolled.
fn actions(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let palette = app.palette;
    let locale = app.locale;
    let sub = palette.text.gamma_multiply(0.65);
    let y = rect.bottom() - 14.0 - ACTIONS / 2.0;
    let full = Rect::from_center_size(pos2(rect.center().x, y), Vec2::splat(32.0));
    let label = gettext(locale, "Full screen lyrics");
    let response = ui
        .interact(full, Id::new("mcloud-np-full"), Sense::click())
        .on_hover_text(label.as_ref());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    let color = if response.hovered() {
        palette.text
    } else {
        sub
    };
    theme::paint_icon(ui, Icon::McloudExpand, full, 18.0, color);
    theme::focus_ring(ui, &response);
    if response.clicked() {
        app.actions.push(Action::SetLyricsFullscreen(true));
    }
    let loaded = matches!(&app.lyrics, crate::model::Loadable::Loaded(Some(_)));
    if loaded && !app.lyrics_following {
        let follow =
            Rect::from_center_size(pos2(rect.right() - PADDING - 40.0, y), vec2(80.0, 28.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(follow));
        if theme::pill_button(
            &mut child,
            &palette,
            &crate::i18n::pgettext(locale, "lyrics", "Follow"),
            false,
        )
        .clicked()
        {
            app.lyrics_following = true;
            app.lyrics_line_shown = None;
        }
    }
}

/// The 24pt close button at the top left, on hover.
fn close_button(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let palette = app.palette;
    if !ui.rect_contains_pointer(rect) {
        return;
    }
    let close = Rect::from_min_size(rect.min + vec2(12.0, 10.0), Vec2::splat(24.0));
    let label = gettext(app.locale, "Close");
    let response = ui
        .interact(close, Id::new("mcloud-np-close"), Sense::click())
        .on_hover_text(label.as_ref());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    ui.painter()
        .circle_filled(close.center(), 12.0, palette.text.gamma_multiply(0.18));
    let color = if response.hovered() {
        palette.text
    } else {
        palette.text.gamma_multiply(0.7)
    };
    theme::paint_icon(ui, Icon::McloudClose, close, 12.0, color);
    if response.clicked() {
        app.actions.push(Action::ToggleLyricsPanel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pane_follows_the_window_between_300_and_380() {
        assert_eq!(pane_width(1000.0), 300.0);
        assert_eq!(pane_width(1400.0), 336.0);
        assert_eq!(pane_width(2560.0), 380.0);
    }

    /// The artwork fills the width when there is room, gives way to the lyrics
    /// in a short window, and never vanishes.
    #[test]
    fn the_artwork_gives_way_to_the_lyrics() {
        assert_eq!(art_side(336.0, 900.0, true), 336.0 - 44.0);
        let short = art_side(336.0, 500.0, true);
        assert!((MIN_ART..292.0).contains(&short));
        assert_eq!(art_side(336.0, 200.0, true), MIN_ART);
        assert!(art_side(336.0, 500.0, false) > short);
    }

    #[test]
    fn the_slide_starts_and_ends_in_place() {
        assert_eq!(slide(0.0), 0.0);
        assert_eq!(slide(1.0), 1.0);
        assert!(slide(0.5) > 0.5);
    }
}

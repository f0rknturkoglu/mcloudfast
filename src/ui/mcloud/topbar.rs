//! The toolbar as mcloud lays it over Spotify's (user.css `#global-nav-bar`):
//! back and forward in one capsule on the left, Home and the search field
//! centred in what is left, each a small dark glass shape. The account,
//! notifications and friends live in the sidebar's bottom row instead.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

/// The band the toolbar takes at the top of the page: 12 points above its
/// 50-point row.
pub const HEIGHT: f32 = 62.0;
const TOP: f32 = 12.0;
const ROW: f32 = 50.0;
const SIDE: f32 = 20.0;
const BUTTON: f32 = 36.0;
/// Home, 8pt and the field share at most 620pt (user.css
/// `.am-toolbar-center > div`).
const GROUP_MAX: f32 = 620.0;

/// mcloud's toolbar fill: a dark glass, 58% of #1e1e1e.
fn fill(hovered: bool) -> Color32 {
    if hovered {
        Color32::from_rgba_unmultiplied(0x3a, 0x3a, 0x3a, 179)
    } else {
        Color32::from_rgba_unmultiplied(0x1e, 0x1e, 0x1e, 148)
    }
}

fn ring() -> Stroke {
    Stroke::new(0.5, Color32::from_white_alpha(31))
}

/// Computes the topbar background opacity progress (`0.0..=1.0`) for `scrolled`.
/// On Album, Playlist, and Liked Songs pages, the frosted glass waits until the
/// hero cover has scrolled beneath the topbar (`hero_threshold - HEIGHT`).
pub(crate) fn scroll_progress(page: &Page, scrolled: f32, hero_threshold: Option<f32>) -> f32 {
    if matches!(page, Page::Album(_) | Page::Playlist(_) | Page::LikedSongs) {
        let threshold = hero_threshold.unwrap_or(180.0);
        let start = (threshold - HEIGHT).max(20.0);
        ((scrolled - start) / 24.0).clamp(0.0, 1.0)
    } else {
        (scrolled / 20.0).clamp(0.0, 1.0)
    }
}

/// Soft specular glow along the glass rim nearest the pointer when the cursor
/// approaches within 28pt.
fn paint_cursor_glow(painter: &egui::Painter, rect: Rect) {
    let Some(pointer) = painter.ctx().input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let expanded = rect.expand(28.0);
    if !expanded.contains(pointer) {
        return;
    }
    let dx = (rect.left() - pointer.x)
        .max(0.0)
        .max(pointer.x - rect.right());
    let dy = (rect.top() - pointer.y)
        .max(0.0)
        .max(pointer.y - rect.bottom());
    let dist = dx.hypot(dy);
    let proximity = (1.0 - dist / 28.0).clamp(0.0, 1.0);
    if proximity <= 0.0 {
        return;
    }
    let clipped = painter.with_clip_rect(rect);
    for (radius, peak_alpha) in [(28.0, 14.0), (18.0, 22.0), (10.0, 28.0)] {
        let alpha = (peak_alpha * proximity).round() as u8;
        clipped.circle_filled(pointer, radius, Color32::from_white_alpha(alpha));
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let inset = if app.settings.sidebar_visible {
        0.0
    } else {
        theme::titlebar_inset(ui.ctx())
    };
    let (band, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT + inset), Sense::hover());
    if fastframe_titlebar::enabled() {
        fastframe_titlebar::drag(ui, band);
    }

    let scrolled_id = Id::new(("page-scrolled", app.page().encode()));
    let (scrolled, hero_threshold) = ui.ctx().data(|data| {
        (
            data.get_temp::<f32>(scrolled_id).unwrap_or(0.0),
            data.get_temp::<f32>(Id::new(super::pages::HERO_SCROLL_THRESHOLD_ID)),
        )
    });
    let progress = scroll_progress(app.page(), scrolled, hero_threshold);

    if progress > 0.0 {
        let _glass = super::glass::paint(ui, band, app.settings.mcloud_glass);
        let fill_color =
            Color32::from_rgba_unmultiplied(0x2a, 0x2a, 0x2a, (158.0 * progress) as u8);
        ui.painter().rect_filled(band, 0.0, fill_color);
        ui.painter().hline(
            band.x_range(),
            band.bottom(),
            Stroke::new(1.0, Color32::from_white_alpha((18.0 * progress) as u8)),
        );
    }

    // A soft shade from the top keeps the controls readable over bright
    // pages.
    crate::ui::widgets::paint_vertical_gradient(
        ui,
        band,
        Color32::from_black_alpha(43),
        Color32::TRANSPARENT,
    );
    let row = Rect::from_min_size(
        pos2(band.left() + SIDE, band.top() + inset + TOP),
        vec2(band.width() - 2.0 * SIDE, ROW),
    );
    let y = row.center().y;
    let mut left = row.left() + 10.0;
    if !app.settings.sidebar_visible {
        let rect = Rect::from_center_size(pos2(left + BUTTON / 2.0, y), Vec2::splat(BUTTON));
        let label = crate::ui::keys::platform_shortcut(
            &gettext(locale, "Show sidebar (Ctrl+B)"),
            &gettext(locale, "Show sidebar (Cmd+B)"),
        )
        .to_owned();
        if circle(
            ui,
            rect,
            Icon::PanelLeft,
            true,
            &label,
            app.settings.mcloud_glass,
        )
        .clicked()
        {
            app.actions.push(Action::ToggleSidebar);
        }
        left = rect.right() + 12.0;
    }

    // Back and forward share one capsule, each lighting its own half.
    let capsule = Rect::from_min_size(pos2(left, y - BUTTON / 2.0), vec2(2.0 * BUTTON, BUTTON));
    let left_half = Rect::from_min_max(capsule.min, pos2(capsule.center().x, capsule.bottom()));
    let right_half = Rect::from_min_max(pos2(capsule.center().x, capsule.top()), capsule.max);
    let pointer_down = ui.ctx().input(|i| {
        i.pointer
            .primary_down()
            .then(|| i.pointer.interact_pos())
            .flatten()
    });
    let capsule_pressed = pointer_down.is_some_and(|pos| {
        (app.can_go_back() && left_half.contains(pos))
            || (app.can_go_forward() && right_half.contains(pos))
    });
    let capsule_scale = ui.ctx().animate_value_with_time(
        Id::new("mcloud-history-capsule-scale"),
        if capsule_pressed { 1.06 } else { 1.0 },
        crate::motion::seconds(ui.ctx(), 0.16),
    );
    let drawn_capsule = Rect::from_center_size(capsule.center(), capsule.size() * capsule_scale);
    let drawn_radius = CornerRadius::same((drawn_capsule.height() / 2.0) as u8);
    let glass = super::glass::paint(ui, drawn_capsule, app.settings.mcloud_glass);
    if !glass {
        ui.painter()
            .rect_filled(drawn_capsule, drawn_radius, fill(false));
    }
    paint_cursor_glow(ui.painter(), drawn_capsule);
    ui.painter()
        .rect_stroke(drawn_capsule, drawn_radius, ring(), StrokeKind::Inside);
    let halves = [
        (
            left_half,
            Icon::McloudLeft,
            app.can_go_back(),
            gettext(locale, "Back"),
            Action::Back,
        ),
        (
            right_half,
            Icon::McloudChevron,
            app.can_go_forward(),
            gettext(locale, "Forward"),
            Action::Forward,
        ),
    ];
    for (rect, icon, enabled, label, action) in halves {
        let response = ui.interact(
            rect,
            Id::new(("mcloud-history", label.as_ref())),
            Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label.as_ref())
        });
        if enabled && response.hovered() {
            ui.painter().rect_filled(
                rect.shrink(2.0),
                CornerRadius::same(((BUTTON - 4.0) / 2.0) as u8),
                Color32::from_white_alpha(22),
            );
        }
        let color = if !enabled {
            Color32::from_white_alpha(77)
        } else if response.hovered() {
            Color32::WHITE
        } else {
            Color32::from_rgb(0xe6, 0xe6, 0xe6)
        };
        theme::paint_icon(ui, icon, rect, 20.0, color);
        if enabled && response.on_hover_text(label.as_ref()).clicked() {
            app.actions.push(action);
        }
    }

    // Home and the search field, centred in what is left of the row.
    let rest = Rect::from_min_max(pos2(capsule.right() + 12.0, row.top()), row.max);
    let search_width = (rest.width() - BUTTON - 8.0 - 10.0).clamp(120.0, GROUP_MAX - BUTTON - 8.0);
    let group = BUTTON + 8.0 + search_width;
    let start = (rest.center().x - group / 2.0).max(rest.left());
    let home = Rect::from_center_size(pos2(start + BUTTON / 2.0, y), Vec2::splat(BUTTON));
    if circle(
        ui,
        home,
        Icon::McloudHome,
        true,
        &gettext(locale, "Home"),
        app.settings.mcloud_glass,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Home));
    }
    let field = Rect::from_min_size(
        pos2(home.right() + 8.0, y - BUTTON / 2.0),
        vec2(search_width, BUTTON),
    );
    let glass = super::glass::paint(ui, field, app.settings.mcloud_glass);
    if !glass {
        ui.painter()
            .rect_filled(field, CornerRadius::same((BUTTON / 2.0) as u8), fill(false));
    }
    paint_cursor_glow(ui.painter(), field);
    ui.painter().rect_stroke(
        field,
        CornerRadius::same((BUTTON / 2.0) as u8),
        ring(),
        StrokeKind::Inside,
    );
    let mut field_ui = ui.new_child(egui::UiBuilder::new().max_rect(field));
    crate::ui::topbar::search_box(app, &mut field_ui, search_width);
    super::search::suggestions_popup(app, ui.ctx(), field);
}

/// A round toolbar button.
fn circle(
    ui: &mut egui::Ui,
    rect: Rect,
    icon: Icon,
    enabled: bool,
    label: &str,
    effect: crate::settings::GlassEffect,
) -> egui::Response {
    let id = Id::new(("mcloud-toolbar", label));
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let hovered = response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let scale = ui.ctx().animate_value_with_time(
        id.with("press-scale"),
        if pressed { 1.08 } else { 1.0 },
        crate::motion::seconds(ui.ctx(), 0.16),
    );
    let drawn = Rect::from_center_size(rect.center(), rect.size() * scale);
    let radius = drawn.width() / 2.0;
    let glass = super::glass::paint(ui, drawn, effect);
    let painter = ui.painter();
    if !glass {
        painter.circle_filled(drawn.center(), radius, fill(hovered));
    } else if hovered {
        painter.circle_filled(drawn.center(), radius, Color32::from_white_alpha(22));
    }
    paint_cursor_glow(painter, drawn);
    painter.circle_stroke(drawn.center(), radius - 0.25, ring());
    let color = if hovered {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xe6, 0xe6, 0xe6)
    };
    theme::paint_icon(ui, icon, drawn, 20.0, color);
    theme::focus_ring(ui, &response);
    response.on_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_and_playlist_topbar_waits_for_hero_cover_to_scroll_past() {
        let album = Page::Album("alb1".into());
        assert_eq!(scroll_progress(&album, 40.0, Some(220.0)), 0.0);
        // Threshold is 220 - 62 = 158; halfway through the 24pt transition is 0.5.
        let mid = scroll_progress(&album, 158.0 + 12.0, Some(220.0));
        assert!((mid - 0.5).abs() < 1e-4);
        assert_eq!(scroll_progress(&album, 200.0, Some(220.0)), 1.0);

        // Non-hero pages still fade in over the first 20pt of scroll.
        assert_eq!(scroll_progress(&Page::Home, 10.0, Some(220.0)), 0.5);
    }
}

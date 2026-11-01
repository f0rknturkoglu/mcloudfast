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
const SEARCH_MAX: f32 = 620.0;

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
    let glass = super::glass::paint(ui, capsule, app.settings.mcloud_glass);
    if !glass {
        ui.painter().rect_filled(
            capsule,
            CornerRadius::same((BUTTON / 2.0) as u8),
            fill(false),
        );
    }
    ui.painter().rect_stroke(
        capsule,
        CornerRadius::same((BUTTON / 2.0) as u8),
        ring(),
        StrokeKind::Inside,
    );
    let halves = [
        (
            Rect::from_min_max(capsule.min, pos2(capsule.center().x, capsule.bottom())),
            Icon::McloudLeft,
            app.can_go_back(),
            gettext(locale, "Back"),
            Action::Back,
        ),
        (
            Rect::from_min_max(pos2(capsule.center().x, capsule.top()), capsule.max),
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
                Color32::from_white_alpha(18),
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
    let search_width = (rest.width() - BUTTON - 8.0 - 10.0).clamp(120.0, SEARCH_MAX);
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
    ui.painter().rect_stroke(
        field,
        CornerRadius::same((BUTTON / 2.0) as u8),
        ring(),
        StrokeKind::Inside,
    );
    let mut field_ui = ui.new_child(egui::UiBuilder::new().max_rect(field));
    crate::ui::topbar::search_box(app, &mut field_ui, search_width);
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
    let response = ui.interact(rect, Id::new(("mcloud-toolbar", label)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let painter = ui.painter();
    let hovered = response.hovered();
    let glass = super::glass::paint(ui, rect, effect);
    if !glass {
        painter.circle_filled(rect.center(), BUTTON / 2.0, fill(hovered));
    } else if hovered {
        painter.circle_filled(rect.center(), BUTTON / 2.0, Color32::from_white_alpha(18));
    }
    painter.circle_stroke(rect.center(), BUTTON / 2.0 - 0.25, ring());
    let color = if response.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xe6, 0xe6, 0xe6)
    };
    theme::paint_icon(ui, icon, rect, 20.0, color);
    theme::focus_ring(ui, &response);
    response.on_hover_text(label)
}

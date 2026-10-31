//! The floating player, as mcloud draws it (user.css `#am-player`): a glass
//! capsule over the page that grows with the window up to [`MAX_WIDTH`],
//! then stays centred. Transport on the left; in the middle the "LCD" with
//! the artwork, the title and its star, the scrolling "Artist — Album" line,
//! the "…" menu and a hairline scrubber along its bottom; mini player,
//! lyrics, devices, queue and volume on the right, where the volume grows
//! out of the speaker as a capsule over the buttons beside it.

use egui::{
    Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder,
    Vec2, pos2, vec2,
};

use crate::app::{App, NowPlaying};
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::player::RepeatMode;
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets;
use crate::util;

pub const HEIGHT: f32 = 54.0;
const MAX_WIDTH: f32 = 700.0;
/// The room the capsule keeps from the page's sides.
const SIDE: f32 = 32.0;
/// The gap between the capsule and the window's bottom edge.
pub const BOTTOM: f32 = 16.0;
const PADDING_LEFT: f32 = 16.0;
const PADDING_RIGHT: f32 = 14.0;
const GAP: f32 = 14.0;
const BUTTON: Vec2 = Vec2::new(28.0, 32.0);
const BUTTON_GAP: f32 = 4.0;
const ICON: f32 = 19.0;
const DIM_ICON: f32 = 14.0;
const PLAY_ICON: f32 = 26.0;
/// The LCD's artwork and words sit between these insets from its top and
/// bottom; the scrubber lives in the bottom one.
const LCD_TOP: f32 = 6.0;
const LCD_BOTTOM: f32 = 13.0;
const VOLUME: Vec2 = Vec2::new(110.0, 38.0);

const NORMAL: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xe8);
const DIM: Color32 = Color32::from_rgb(0x8d, 0x8d, 0x8d);

/// Room a page leaves at its end so its last rows can scroll clear of the
/// capsule.
pub const PAGE_CLEARANCE: f32 = HEIGHT + BOTTOM;

/// Draws the capsule over `content`, the part of the window the page uses.
pub fn show(app: &mut App, ui: &mut egui::Ui, content: Rect) {
    let rect = capsule(content);
    let ctx = ui.ctx().clone();
    egui::Area::new(Id::new("mcloud-player"))
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .constrain(false)
        .show(&ctx, |ui| {
            // Room for the solid surface's shadow.
            ui.set_clip_rect(rect.expand(40.0));
            // The capsule is the area's own: clicks between its controls
            // stay on it rather than reaching the page beneath.
            ui.allocate_exact_size(rect.size(), Sense::click());
            let palette = app.palette;
            surface(ui, rect, app.settings.mcloud_glass);
            let now = app.now_playing();
            let transport_right = transport(app, ui, rect, now.as_ref());
            let extras_left = extras(app, ui, rect, now.as_ref());
            let lcd = Rect::from_min_max(
                pos2(transport_right + GAP, rect.top()),
                pos2(extras_left - GAP, rect.bottom()),
            );
            if lcd.width() > 60.0 {
                display(app, ui, &palette, lcd, now.as_ref());
            }
        });
}

/// Where the capsule sits over `content`: an even width, a whole left edge.
fn capsule(content: Rect) -> Rect {
    let width = ((content.width() - 2.0 * SIDE).clamp(0.0, MAX_WIDTH) / 2.0).floor() * 2.0;
    let left = (content.left() + (content.width() - width) / 2.0).floor();
    let top = (content.bottom() - BOTTOM - HEIGHT).round();
    Rect::from_min_size(pos2(left, top), vec2(width, HEIGHT))
}

/// The capsule itself. The liquid glass carries its own lens, tone and
/// highlight, so nothing is drawn over it; Eco, or a GPU without the glass,
/// gets mcloud's solid surface.
fn surface(ui: &egui::Ui, rect: Rect, effect: crate::settings::GlassEffect) {
    if super::glass::paint(ui, rect, effect) {
        return;
    }
    let corner = CornerRadius::same((HEIGHT / 2.0) as u8);
    let painter = ui.painter();
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(85),
        }
        .as_shape(rect, corner),
    );
    // The gradient's middle colour fills the round ends.
    painter.rect_filled(rect, corner, Color32::from_rgb(0x33, 0x33, 0x35));
    widgets::paint_vertical_gradient(
        ui,
        rect.shrink2(vec2(HEIGHT / 2.0, 0.0)),
        Color32::from_rgb(0x3b, 0x3b, 0x3d),
        Color32::from_rgb(0x2b, 0x2b, 0x2d),
    );
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, Color32::from_white_alpha(18)),
        StrokeKind::Inside,
    );
    painter.hline(
        (rect.left() + HEIGHT / 2.0)..=(rect.right() - HEIGHT / 2.0),
        rect.top() + 1.0,
        Stroke::new(1.0, Color32::from_white_alpha(38)),
    );
}

/// A button of mcloud's player: an icon on no background that whitens under
/// the pointer and shrinks a little while pressed.
fn button(
    ui: &mut egui::Ui,
    rect: Rect,
    icon: Icon,
    size: f32,
    color: Color32,
    label: &str,
) -> egui::Response {
    let response = ui.interact(
        rect,
        Id::new(("mcloud-player-button", label)),
        Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let color = if response.hovered() && color == NORMAL {
        Color32::WHITE
    } else {
        color
    };
    let scale = if response.is_pointer_button_down_on() {
        0.92
    } else {
        1.0
    };
    theme::paint_icon(ui, icon, rect, size * scale, color);
    theme::focus_ring(ui, &response);
    if label.is_empty() {
        response
    } else {
        response.on_hover_text(label)
    }
}

/// The rects of `count` buttons in a row from `left`, centred on `y`.
fn row(left: f32, y: f32, count: usize) -> Vec<Rect> {
    (0..count)
        .map(|index| {
            Rect::from_min_size(
                pos2(
                    left + index as f32 * (BUTTON.x + BUTTON_GAP),
                    y - BUTTON.y / 2.0,
                ),
                BUTTON,
            )
        })
        .collect()
}

fn row_width(count: usize) -> f32 {
    count as f32 * BUTTON.x + (count as f32 - 1.0) * BUTTON_GAP
}

/// Shuffle, back, play, forward and repeat. Returns their right edge.
fn transport(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let playing = now.is_some_and(|now| now.playing);
    let loading = now.is_some_and(|now| now.loading) || app.any_play_pending();
    let shuffle = now.map_or_else(|| app.playing_context_shuffle(), |now| now.shuffle);
    let repeat = now.map(|now| now.repeat).unwrap_or_default();
    let slots = row(rect.left() + PADDING_LEFT, rect.center().y, 5);

    let color = if shuffle { palette.accent } else { DIM };
    let response = button(
        ui,
        slots[0],
        Icon::McloudShuffle,
        DIM_ICON,
        color,
        &gettext(locale, "Shuffle"),
    );
    if response.clicked() {
        app.actions.push(Action::ToggleShuffle);
    }

    if button(
        ui,
        slots[1],
        Icon::McloudBack,
        ICON,
        NORMAL,
        &gettext(locale, "Previous"),
    )
    .clicked()
    {
        app.actions.push(Action::Previous);
    }

    if loading {
        let mut cell = ui.new_child(
            UiBuilder::new()
                .max_rect(slots[2])
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        theme::spinner(&mut cell, 18.0, NORMAL);
    } else {
        let (icon, label) = if playing {
            (Icon::McloudPause, gettext(locale, "Pause"))
        } else {
            (Icon::McloudPlay, gettext(locale, "Play"))
        };
        if button(ui, slots[2], icon, PLAY_ICON, NORMAL, &label).clicked() {
            app.actions.push(Action::TogglePlay);
        }
    }

    if button(
        ui,
        slots[3],
        Icon::McloudNext,
        ICON,
        NORMAL,
        &gettext(locale, "Next"),
    )
    .clicked()
    {
        app.actions.push(Action::Next);
    }

    let (icon, color, label) = match repeat {
        RepeatMode::Off => (Icon::McloudRepeat, DIM, gettext(locale, "Repeat")),
        RepeatMode::Context => (
            Icon::McloudRepeat,
            palette.accent,
            gettext(locale, "Repeat one"),
        ),
        RepeatMode::Track => (Icon::Repeat1, palette.accent, gettext(locale, "Repeat off")),
    };
    if button(ui, slots[4], icon, DIM_ICON, color, &label).clicked() {
        app.actions.push(Action::CycleRepeat);
    }
    slots[4].right()
}

/// Mini player, lyrics, devices, queue and volume, from the capsule's
/// right. Returns the group's left edge.
fn extras(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let left = rect.right() - PADDING_RIGHT - row_width(5);
    let slots = row(left, rect.center().y, 5);
    let speaker = slots[4];

    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown = match app.volume_preview {
        Some(fraction) => (fraction * 100.0).round() as u8,
        None => volume,
    };
    let adjustable = !app.soloist_waiting() && now.is_none_or(|now| now.can_set_volume);
    // The capsule grows leftward out of the speaker, 8 points past it.
    let capsule = Rect::from_min_size(
        pos2(
            speaker.right() + 8.0 - VOLUME.x,
            speaker.center().y - VOLUME.y / 2.0,
        ),
        VOLUME,
    );
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let open_id = Id::new("mcloud-volume-open");
    let was_open = ui.data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    let hovering = pointer
        .is_some_and(|at| speaker.contains(at) || (was_open && capsule.expand(4.0).contains(at)));
    let open = adjustable && (hovering || app.volume_preview.is_some());
    ui.data_mut(|data| data.insert_temp(open_id, open));
    let reveal = ui.ctx().animate_bool_with_time(open_id, open, 0.35);

    // The buttons beside the speaker fade as the capsule covers them.
    let faded = |color: Color32| color.gamma_multiply(1.0 - reveal);
    if reveal < 1.0 {
        let enabled = !open;
        let mini = app.settings.winamp_window;
        let response = button(
            ui,
            slots[0],
            Icon::McloudPopup,
            ICON,
            faded(if mini { palette.accent } else { NORMAL }),
            &gettext(locale, "Mini player"),
        );
        if response.clicked() && enabled {
            app.actions.push(Action::ToggleWinampWindow);
        }

        let lyrics_open = app.show_lyrics_panel || app.lyrics_fullscreen.is_some();
        let response = button(
            ui,
            slots[1],
            Icon::McloudLyrics,
            ICON,
            faded(if lyrics_open { palette.accent } else { NORMAL }),
            &gettext(locale, "Lyrics"),
        );
        if response.clicked() && enabled {
            app.actions.push(Action::ToggleLyricsPanel);
        }

        let remote = now.is_some_and(|now| !now.local) && !app.targets_own_soloist();
        let response = button(
            ui,
            slots[2],
            Icon::McloudDevices,
            ICON,
            faded(if remote { palette.accent } else { NORMAL }),
            &gettext(locale, "Connect to a device"),
        );
        ui.ctx().data_mut(|data| {
            data.insert_temp(Id::new(crate::ui::devices::BUTTON_RECT_ID), response.rect)
        });
        if response.clicked() && enabled {
            app.actions.push(Action::ToggleDevicesPopup);
        }

        let queue_open = app.show_queue_panel || matches!(app.page(), Page::Queue);
        let response = button(
            ui,
            slots[3],
            Icon::McloudQueue,
            ICON,
            faded(if queue_open { palette.accent } else { NORMAL }),
            &gettext(locale, "Queue"),
        );
        if response.clicked() && enabled {
            app.actions.push(Action::ToggleQueuePanel);
        }
    }

    if reveal > 0.0 {
        volume_capsule(app, ui, capsule, reveal, shown, now);
    }

    let icon = match shown {
        0 => Icon::VolumeX,
        1..=33 => Icon::McloudVolume1,
        34..=66 => Icon::McloudVolume2,
        _ => Icon::McloudVolume,
    };
    let label = if shown == 0 {
        gettext(locale, "Unmute")
    } else {
        gettext(locale, "Mute")
    };
    let response = button(ui, speaker, icon, ICON, NORMAL, &label);
    if adjustable {
        // The wheel steps the volume as the ear hears it: in equal steps of
        // its square root, never less than a point.
        let notches = widgets::wheel_notches(ui, &response);
        if notches != 0 {
            let level = f32::from(shown) / 100.0;
            let next = (level.sqrt() + notches as f32 * 0.05)
                .clamp(0.0, 1.0)
                .powi(2);
            let next = if notches > 0 {
                next.max(level + 0.01)
            } else {
                next.min(level - 0.01)
            };
            let percent = (next.clamp(0.0, 1.0) * 100.0).round() as u8;
            app.actions.push(Action::SetVolume(percent));
        }
        if response.clicked() {
            app.actions.push(Action::ToggleMute);
        }
    }
    left
}

/// The volume's capsule: a lighter glass pill with a thick track, revealed
/// from its right end as `reveal` grows.
fn volume_capsule(
    app: &mut App,
    ui: &mut egui::Ui,
    capsule: Rect,
    reveal: f32,
    shown: u8,
    now: Option<&NowPlaying>,
) {
    let visible = Rect::from_min_max(
        pos2(
            capsule.right() - VOLUME.y - (capsule.width() - VOLUME.y) * reveal,
            capsule.top(),
        ),
        capsule.max,
    );
    let corner = CornerRadius::same((VOLUME.y / 2.0) as u8);
    let painter = ui.painter();
    painter.rect_filled(
        visible,
        corner,
        Color32::from_white_alpha((26.0 * reveal) as u8),
    );
    painter.rect_stroke(
        visible,
        corner,
        Stroke::new(1.0, Color32::from_white_alpha((31.0 * reveal) as u8)),
        StrokeKind::Inside,
    );
    painter.hline(
        (visible.left() + VOLUME.y / 2.0)..=(visible.right() - VOLUME.y / 2.0),
        visible.top() + 1.0,
        Stroke::new(1.0, Color32::from_white_alpha((71.0 * reveal) as u8)),
    );
    // The track, between 16 points from the left and 40 from the right.
    let track = Rect::from_min_max(
        pos2(capsule.left() + 16.0, capsule.center().y - 4.0),
        pos2(capsule.right() - 40.0, capsule.center().y + 4.0),
    );
    if track.left() < visible.left() {
        return;
    }
    let response = ui.interact(
        track.expand2(vec2(4.0, 8.0)),
        Id::new("mcloud-volume-track"),
        Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::slider(
            true,
            f64::from(shown),
            gettext(app.locale, "Volume (%)").as_ref(),
        )
    });
    let fraction_at = |at: Pos2| ((at.x - track.left()) / track.width()).clamp(0.0, 1.0);
    if response.dragged()
        && let Some(at) = response.interact_pointer_pos()
    {
        let value = fraction_at(at);
        app.volume_preview = Some(value);
        if now.is_none_or(|now| now.local) {
            app.actions
                .push(Action::PreviewVolume((value * 100.0).round() as u8));
        }
    }
    if (response.drag_stopped() || response.clicked())
        && let Some(at) = response.interact_pointer_pos()
    {
        app.volume_preview = None;
        app.actions
            .push(Action::SetVolume((fraction_at(at) * 100.0).round() as u8));
    }
    let level = f32::from(shown) / 100.0;
    let track_corner = CornerRadius::same(4);
    painter.rect_filled(
        track,
        track_corner,
        Color32::from_white_alpha((77.0 * reveal) as u8),
    );
    let mut filled = track;
    filled.set_right(track.left() + track.width() * level);
    painter.rect_filled(filled, track_corner, Color32::WHITE.gamma_multiply(reveal));
}

/// Words over the glass carry mcloud's soft shadow (0 1px 6px #0008),
/// approximated by a faint copy a point lower.
fn shadowed(ui: &egui::Ui, pos: Pos2, galley: std::sync::Arc<egui::Galley>, color: Color32) {
    ui.painter().galley(
        pos + vec2(0.0, 1.0),
        galley.clone(),
        Color32::from_black_alpha(90),
    );
    ui.painter().galley(pos, galley, color);
}

/// The LCD: artwork, title and star, the scrolling subtitle, "…", and the
/// scrubber along its bottom.
fn display(
    app: &mut App,
    ui: &mut egui::Ui,
    palette: &Palette,
    lcd: Rect,
    now: Option<&NowPlaying>,
) {
    let locale = app.locale;
    let inner = Rect::from_min_max(
        pos2(lcd.left(), lcd.top() + LCD_TOP),
        pos2(lcd.right(), lcd.bottom() - LCD_BOTTOM),
    );
    let Some(now) = now else {
        theme::paint_icon(
            ui,
            Icon::McloudMusic,
            lcd.with_max_y(lcd.bottom() - 12.0),
            22.0,
            Color32::from_rgb(0x96, 0x96, 0x96),
        );
        return;
    };

    // The scrubber's strip, the LCD's bottom 17 points. Under the pointer
    // the bar thickens, the LCD fades and the times show.
    let strip = Rect::from_min_max(pos2(lcd.left(), lcd.bottom() - 17.0), lcd.max);
    let scrub = ui.interact(
        strip,
        Id::new("mcloud-player-scrubber"),
        Sense::click_and_drag(),
    );
    let scrubbing = ui.ctx().animate_bool_with_time(
        Id::new("mcloud-player-scrubbing"),
        scrub.hovered() || scrub.dragged(),
        0.2,
    );
    let fade = 1.0 - 0.8 * scrubbing;

    let side = inner.height();
    let art = Rect::from_min_size(inner.min, Vec2::splat(side));
    let art_response = ui.interact(art, Id::new("mcloud-player-art"), Sense::click());
    ui.painter().add(
        egui::epaint::Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: Color32::from_black_alpha((102.0 * fade) as u8),
        }
        .as_shape(art, CornerRadius::same(5)),
    );
    widgets::paint_cover(
        ui,
        palette,
        now.art_small.as_deref().or(now.art_url.as_deref()),
        art,
        5.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    if fade < 1.0 {
        ui.painter().rect_filled(
            art,
            CornerRadius::same(5),
            Color32::from_black_alpha((255.0 * (1.0 - fade) * 0.8) as u8),
        );
    }
    if art_response.hovered() && scrubbing == 0.0 {
        ui.painter()
            .rect_filled(art, CornerRadius::same(5), Color32::from_black_alpha(140));
        theme::paint_icon(ui, Icon::McloudExpand, art, side * 0.55, Color32::WHITE);
    }
    if art_response
        .on_hover_text(gettext(locale, "Full screen lyrics"))
        .clicked()
    {
        app.actions.push(Action::SetLyricsFullscreen(true));
    }

    // "…" at the LCD's end.
    let more = Rect::from_center_size(pos2(lcd.right() - BUTTON.x / 2.0, inner.center().y), BUTTON);
    if let Some(item) = app.now_playing_item() {
        let response = button(
            ui,
            more,
            Icon::McloudMore,
            20.0,
            Color32::from_rgb(0xe0, 0xe0, 0xe0).gamma_multiply(fade),
            &gettext(locale, "More"),
        );
        let context = app.editable_context_playlist();
        egui::Popup::menu(&response)
            .frame(widgets::menu_frame(palette))
            .align(egui::RectAlign::TOP_END)
            .show(|ui| widgets::item_menu(ui, app, &item, context.as_ref(), None));
    }

    let text_left = art.right() + 9.0;
    let text_right = more.left() - 4.0;
    let width = (text_right - text_left).max(0.0);
    let title_color = Color32::from_rgb(0xf5, 0xf5, 0xf5).gamma_multiply(fade);
    let title = widgets::ellipsized(
        ui,
        &now.title,
        theme::semibold(13.0),
        title_color,
        (width - 19.0).max(0.0),
        1,
    );
    let title_rect = Rect::from_min_size(
        pos2(text_left, inner.center().y - 1.0 - title.size().y),
        title.size(),
    );
    let title_response = ui
        .interact(title_rect, Id::new("mcloud-player-title"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    shadowed(ui, title_rect.min, title, title_color);
    if title_response.clicked() {
        if let Some(id) = &now.album_id {
            app.actions.push(Action::Open(Page::Album(id.clone())));
        } else if let Some(id) = &now.show_id {
            app.actions.push(Action::Open(Page::Show(id.clone())));
        }
    }

    // The star follows the title, shown on the LCD's hover or once starred.
    if !now.is_episode {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        let lcd_hovered = ui.rect_contains_pointer(inner);
        if saved || lcd_hovered {
            let star = Rect::from_center_size(
                pos2(title_rect.right() + 6.0 + 6.5, title_rect.center().y),
                Vec2::splat(15.0),
            );
            let response = ui.interact(star, Id::new("mcloud-player-star"), Sense::click());
            let color = if saved {
                palette.accent
            } else {
                Color32::from_rgb(0x9a, 0x9a, 0x9a)
            };
            theme::paint_icon(ui, Icon::McloudStar, star, 13.0, color.gamma_multiply(fade));
            let label = if saved {
                gettext(locale, "Remove from Liked Songs")
            } else {
                gettext(locale, "Save to Liked Songs")
            };
            if response.on_hover_text(label).clicked() {
                app.actions.push(Action::ToggleSaved(now.uri.clone()));
            }
        }
    }

    let subtitle_rect = Rect::from_min_max(
        pos2(text_left, inner.center().y + 1.0),
        pos2(text_right, inner.center().y + 18.0),
    );
    let subtitle_response = ui
        .interact(
            subtitle_rect,
            Id::new("mcloud-player-subtitle"),
            Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    marquee(
        ui,
        &subtitle(now),
        subtitle_rect,
        Color32::from_white_alpha(208).gamma_multiply(fade),
        ui.rect_contains_pointer(lcd),
    );
    if subtitle_response.clicked()
        && let Some(id) = now.artists.first().and_then(|artist| artist.id.clone())
    {
        app.actions.push(Action::Open(Page::Artist(id)));
    }

    let (position, duration) = (now.position_ms, now.duration_ms);
    let shown_position = app
        .seek_preview
        .map_or(position, |fraction| (fraction * duration as f32) as u32);
    if scrubbing > 0.0 {
        let color = Color32::from_rgb(0xd4, 0xd4, 0xd4).gamma_multiply(scrubbing);
        let y = lcd.bottom() - 19.0 - 8.0;
        ui.painter().text(
            pos2(lcd.left() + 1.0, y),
            Align2::LEFT_CENTER,
            util::format_duration_ms(shown_position),
            theme::semibold(13.0),
            color,
        );
        ui.painter().text(
            pos2(lcd.right() - 1.0, y),
            Align2::RIGHT_CENTER,
            format!(
                "-{}",
                util::format_duration_ms(duration.saturating_sub(shown_position))
            ),
            theme::semibold(13.0),
            color,
        );
    }
    scrubber(app, ui, &scrub, lcd, scrubbing, duration, shown_position);
}

/// "Artist — Album", or a podcast's subtitle.
fn subtitle(now: &NowPlaying) -> String {
    let artists = now
        .artists
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let lead = if artists.is_empty() {
        now.subtitle.clone()
    } else {
        artists
    };
    if now.album_name.is_empty() || now.is_episode {
        lead
    } else {
        format!("{lead} — {}", now.album_name)
    }
}

/// Text that fits stands still; text that does not scrolls while the
/// pointer is on the LCD and otherwise ends with an ellipsis, so an idle
/// window does not repaint for it.
fn marquee(ui: &egui::Ui, text: &str, rect: Rect, color: Color32, moving: bool) {
    const GAP: f32 = 48.0;
    const SPEED: f32 = 28.0;
    const PAUSE: f64 = 1.2;
    let font = theme::regular(13.0);
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let left = |galley: &std::sync::Arc<egui::Galley>| {
        pos2(rect.left(), rect.center().y - galley.size().y / 2.0)
    };
    let start_id = Id::new("mcloud-marquee-start");
    if full.size().x <= rect.width() || !moving {
        let galley = widgets::ellipsized(ui, text, font, color, rect.width(), 1);
        shadowed(ui, left(&galley), galley, color);
        ui.ctx().data_mut(|data| data.remove::<f64>(start_id));
        return;
    }
    let now = ui.input(|input| input.time);
    let start = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(start_id, || now));
    let elapsed = (now - start - PAUSE).max(0.0) as f32;
    let cycle = full.size().x + GAP;
    let offset = (elapsed * SPEED) % cycle;
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let origin = left(&full) - vec2(offset, 0.0);
    painter.galley(origin, full.clone(), color);
    painter.galley(origin + vec2(cycle, 0.0), full, color);
    ui.ctx().request_repaint();
}

/// The hairline along the LCD's bottom: 3 points, 9 while scrubbing, and a
/// click or a drag anywhere along the strip seeks.
fn scrubber(
    app: &mut App,
    ui: &egui::Ui,
    response: &egui::Response,
    lcd: Rect,
    scrubbing: f32,
    duration: u32,
    position: u32,
) {
    let fraction_at = |x: f32| ((x - lcd.left()) / lcd.width()).clamp(0.0, 1.0);
    if duration > 0 {
        if response.dragged()
            && let Some(at) = response.interact_pointer_pos()
        {
            app.seek_preview = Some(fraction_at(at.x));
        }
        if (response.drag_stopped() || response.clicked())
            && let Some(at) = response.interact_pointer_pos()
        {
            app.seek_preview = None;
            app.actions
                .push(Action::Seek((fraction_at(at.x) * duration as f32) as u32));
        }
    }
    let fraction = if duration > 0 {
        position as f32 / duration as f32
    } else {
        0.0
    };
    let height = 3.0 + 6.0 * scrubbing;
    let bottom = lcd.bottom() - 6.0 + scrubbing;
    let track = Rect::from_min_max(pos2(lcd.left(), bottom - height), pos2(lcd.right(), bottom));
    let corner = CornerRadius::same((height / 2.0).round() as u8);
    let painter = ui.painter();
    painter.rect_filled(track, corner, Color32::from_white_alpha(51));
    let mut filled = track;
    filled.set_right(track.left() + track.width() * fraction);
    painter.rect_filled(filled, corner, Color32::from_rgb(0xe9, 0xe9, 0xe9));
    response.widget_info(|| {
        egui::WidgetInfo::slider(
            ui.is_enabled(),
            f64::from(fraction * 100.0),
            gettext(app.locale, "Playback position (%)"),
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capsule_grows_with_the_page_up_to_its_width_and_stays_centred() {
        let content = |width: f32| Rect::from_min_size(pos2(250.0, 0.0), vec2(width, 800.0));
        assert_eq!(capsule(content(600.0)).width(), 536.0);
        let wide = capsule(content(1600.0));
        assert_eq!(wide.width(), MAX_WIDTH);
        assert_eq!(wide.center().x, content(1600.0).center().x);
        assert_eq!(wide.bottom(), 800.0 - BOTTOM);
        // An odd room rounds down to an even width, on a whole left edge.
        let odd = capsule(content(601.0));
        assert_eq!(odd.width() % 2.0, 0.0);
        assert_eq!(odd.left().fract(), 0.0);
    }
}

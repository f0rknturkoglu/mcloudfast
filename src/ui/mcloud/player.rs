//! The floating player, as mcloud draws it (user.css `#am-player`): a glass
//! capsule over the page that grows with the window up to `MAX_WIDTH`,
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
/// How long the pointer rests on the speaker before the volume capsule opens.
const VOLUME_HOVER_DELAY: f64 = 0.5;

/// The player's icons, mcloud's `--am-icon` and `--am-dim`.
fn normal() -> Color32 {
    super::tone(
        Color32::from_rgb(0xe8, 0xe8, 0xe8),
        Color32::from_rgb(0x2b, 0x2b, 0x2b),
    )
}

fn dim() -> Color32 {
    super::tone(
        Color32::from_rgb(0x8d, 0x8d, 0x8d),
        Color32::from_rgb(0xa1, 0xa1, 0xa6),
    )
}

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
            let (_, capsule_response) = ui.allocate_exact_size(rect.size(), Sense::click());
            let palette = app.palette;
            if app.settings.mcloud_player_menu_anywhere
                && let Some(item) = app.now_playing_item()
            {
                let context = app.editable_context_playlist();
                egui::Popup::context_menu(&capsule_response)
                    .frame(widgets::menu_frame(&palette))
                    .show(|ui| widgets::item_menu(ui, app, &item, context.as_ref(), None));
            }
            let effect = if app.settings.mcloud_player_glass {
                app.settings.mcloud_glass
            } else {
                crate::settings::GlassEffect::Eco
            };
            surface(ui, rect, effect, app.lyrics_fullscreen.is_some());
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

/// Every control of the player for Now Playing alone: the scrubber, the
/// transport under it, and the other buttons under that.
pub(in crate::ui) fn panel_controls(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let now = app.now_playing();
    let strip = Rect::from_min_size(rect.min, vec2(rect.width(), 17.0));
    if let Some(now) = &now {
        let scrub = ui.interact(
            strip,
            Id::new("mcloud-np-scrubber"),
            Sense::click_and_drag(),
        );
        let scrubbing = ui.ctx().animate_bool_with_time(
            Id::new("mcloud-np-scrubbing"),
            scrub.hovered() || scrub.dragged(),
            0.2,
        );
        let duration = now.duration_ms;
        let shown_position = app.seek_preview.map_or(now.position_ms, |fraction| {
            (fraction * duration as f32) as u32
        });
        let lcd = strip.with_min_y(strip.bottom() - 17.0);
        scrubber(app, ui, &scrub, lcd, scrubbing, duration, shown_position);
    }
    // The groups lay themselves out as they do in a wide capsule, centred.
    let transport_row = Rect::from_center_size(
        pos2(
            rect.center().x - row_width(5) / 2.0 - PADDING_LEFT + 300.0,
            strip.bottom() + 20.0,
        ),
        vec2(600.0, HEIGHT),
    );
    transport(app, ui, transport_row, now.as_ref());
    let extras_row = Rect::from_center_size(
        pos2(
            rect.center().x + row_width(6) / 2.0 + PADDING_RIGHT - 300.0,
            strip.bottom() + 54.0,
        ),
        vec2(600.0, HEIGHT),
    );
    extras(app, ui, extras_row, now.as_ref());
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
fn surface(ui: &egui::Ui, rect: Rect, effect: crate::settings::GlassEffect, over_lyrics: bool) {
    if super::glass::paint_surface(ui, rect, effect, over_lyrics) {
        return;
    }
    solid_capsule(ui, rect);
}

/// mcloud's solid capsule surface when Eco glass or reduced transparency is
/// active, or the GPU cannot run the glass shaders.
pub(in crate::ui) fn solid_capsule(ui: &egui::Ui, rect: Rect) {
    let half = rect.height() / 2.0;
    let corner = CornerRadius::same(half.min(255.0) as u8);
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
    let top = crate::ui::mcloud::tone(
        Color32::from_rgb(0x3b, 0x3b, 0x3d),
        Color32::from_rgb(0xfd, 0xfd, 0xfd),
    );
    let bottom = crate::ui::mcloud::tone(
        Color32::from_rgb(0x2b, 0x2b, 0x2d),
        Color32::from_rgb(0xec, 0xec, 0xec),
    );
    // The fill only gives the outline its antialiased edge; the gradient runs
    // over the whole capsule, the round ends included.
    painter.rect_filled(rect, corner, top.lerp_to_gamma(bottom, 0.5));
    painter.add(egui::Shape::mesh(capsule_gradient(rect, top, bottom)));
    // mcloud's --am-rim is white in both appearances.
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, Color32::from_white_alpha(18)),
        StrokeKind::Inside,
    );
    painter.hline(
        (rect.left() + half)..=(rect.right() - half),
        rect.top() + 1.0,
        Stroke::new(1.0, Color32::from_white_alpha(38)),
    );
}

/// A top-to-bottom gradient in the shape of the capsule `rect`, half a point
/// inside its edge. A mesh has no antialiasing, so whatever is under it draws
/// that edge.
fn capsule_gradient(rect: Rect, top: Color32, bottom: Color32) -> egui::Mesh {
    const EDGE: f32 = 0.5;
    let radius = rect.height() / 2.0;
    let rows = (rect.height() * 2.0).ceil().max(1.0) as u32;
    let mut mesh = egui::Mesh::default();
    for row in 0..=rows {
        let y = (rect.top() + EDGE) + (rect.height() - 2.0 * EDGE) * row as f32 / rows as f32;
        let from_middle = y - rect.center().y;
        let reach = (radius * radius - from_middle * from_middle)
            .max(0.0)
            .sqrt();
        let inset = (radius - reach + EDGE).min(rect.width() / 2.0);
        let colour = top.lerp_to_gamma(bottom, (y - rect.top()) / rect.height());
        mesh.colored_vertex(egui::pos2(rect.left() + inset, y), colour);
        mesh.colored_vertex(egui::pos2(rect.right() - inset, y), colour);
        if row > 0 {
            let at = row * 2;
            mesh.add_triangle(at - 2, at - 1, at);
            mesh.add_triangle(at - 1, at, at + 1);
        }
    }
    mesh
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
    let color = if response.hovered() && color == normal() {
        super::tone(Color32::WHITE, Color32::BLACK)
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

/// Below this capsule width, secondary controls hide so the central LCD
/// retains room for song title, subtitle, and scrub bar.
const NARROW_CAPSULE: f32 = 560.0;

/// Shuffle, back, play, forward and repeat. Returns their right edge.
fn transport(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let playing = now.is_some_and(|now| now.playing);
    let loading = now.is_some_and(|now| now.loading) || app.any_play_pending();
    let shuffle = now.map_or_else(|| app.playing_context_shuffle(), |now| now.shuffle);
    let repeat = now.map(|now| now.repeat).unwrap_or_default();
    let narrow = rect.width() < NARROW_CAPSULE;
    let ids = shown_buttons(
        if narrow {
            &["previous", "play", "next"]
        } else {
            &["shuffle", "previous", "play", "next", "repeat"]
        },
        &app.settings.mcloud_player_hidden,
    );
    let slots = row(rect.left() + PADDING_LEFT, rect.center().y, ids.len());
    let slot = |id: &str| {
        ids.iter()
            .position(|shown| *shown == id)
            .map(|index| slots[index])
    };
    let play_slot = slot("play").expect("play is never hidden");

    if let Some(shuffle_slot) = slot("shuffle") {
        let color = if shuffle { palette.accent } else { dim() };
        let response = button(
            ui,
            shuffle_slot,
            Icon::McloudShuffle,
            DIM_ICON,
            color,
            &gettext(locale, "Shuffle"),
        );
        if response.clicked() {
            app.actions.push(Action::ToggleShuffle);
        }
    }

    if let Some(back_slot) = slot("previous")
        && button(
            ui,
            back_slot,
            Icon::McloudBack,
            ICON,
            normal(),
            &gettext(locale, "Previous"),
        )
        .clicked()
    {
        app.actions.push(Action::Previous);
    }

    if loading {
        let mut cell = ui.new_child(
            UiBuilder::new()
                .max_rect(play_slot)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        theme::spinner(&mut cell, 18.0, normal());
    } else {
        let (icon, label) = if playing {
            (Icon::McloudPause, gettext(locale, "Pause"))
        } else {
            (Icon::McloudPlay, gettext(locale, "Play"))
        };
        if button(ui, play_slot, icon, PLAY_ICON, normal(), &label).clicked() {
            app.actions.push(Action::TogglePlay);
        }
    }

    if let Some(next_slot) = slot("next")
        && button(
            ui,
            next_slot,
            Icon::McloudNext,
            ICON,
            normal(),
            &gettext(locale, "Next"),
        )
        .clicked()
    {
        app.actions.push(Action::Next);
    }

    if let Some(repeat_slot) = slot("repeat") {
        let (icon, color, label) = match repeat {
            RepeatMode::Off => (Icon::McloudRepeat, dim(), gettext(locale, "Repeat")),
            RepeatMode::Context => (
                Icon::McloudRepeat,
                palette.accent,
                gettext(locale, "Repeat one"),
            ),
            RepeatMode::Track => (Icon::Repeat1, palette.accent, gettext(locale, "Repeat off")),
        };
        if button(ui, repeat_slot, icon, DIM_ICON, color, &label).clicked() {
            app.actions.push(Action::CycleRepeat);
        }
    }
    slots.last().map_or(rect.left(), |last| last.right())
}

/// The player buttons a user can turn off, for Settings.
pub(in crate::ui) const HIDEABLE_BUTTONS: [(&str, &str); 10] = [
    ("shuffle", "Shuffle"),
    ("previous", "Previous"),
    ("next", "Next"),
    ("repeat", "Repeat"),
    ("mini", "Mini player"),
    ("lyrics", "Lyrics"),
    ("nowplaying", "Now Playing"),
    ("devices", "Connect to a device"),
    ("queue", "Queue"),
    ("volume", "Mute"),
];

#[cfg(test)]
mod button_tests {
    use super::*;

    #[test]
    fn turned_off_buttons_leave_but_play_stays() {
        let hidden = vec!["next".to_string(), "play".to_string()];
        assert_eq!(
            shown_buttons(&["previous", "play", "next"], &hidden),
            ["previous", "play"]
        );
    }

    #[test]
    fn only_the_digits_that_changed_roll() {
        assert_eq!(changed_digits("1:09", "1:10"), [false, false, true, true]);
        // Aligned at the end, so a new leading digit is the only new column.
        assert_eq!(
            changed_digits("9:59", "10:00"),
            [true, true, false, true, true]
        );
    }
}

/// The buttons of a group left once the turned-off ones go. Play always stays.
fn shown_buttons(group: &[&'static str], hidden: &[String]) -> Vec<&'static str> {
    group
        .iter()
        .copied()
        .filter(|id| *id == "play" || !hidden.iter().any(|name| name == id))
        .collect()
}

/// Mini player, lyrics, devices, queue and volume, from the capsule's
/// right. Returns the group's left edge.
fn extras(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let narrow = rect.width() < NARROW_CAPSULE;
    let ids = shown_buttons(
        if narrow {
            &["nowplaying", "queue", "volume"]
        } else {
            &["mini", "lyrics", "nowplaying", "devices", "queue", "volume"]
        },
        &app.settings.mcloud_player_hidden,
    );
    let count = ids.len();
    if count == 0 {
        return rect.right() - PADDING_RIGHT;
    }
    let left = rect.right() - PADDING_RIGHT - row_width(count);
    let slots = row(left, rect.center().y, count);
    let slot = |id: &str| {
        ids.iter()
            .position(|shown| *shown == id)
            .map(|index| slots[index])
    };
    let speaker_slot = slot("volume");
    let speaker = speaker_slot.unwrap_or(slots[count - 1]);

    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown = match app.volume_preview {
        Some(fraction) => (fraction * 100.0).round() as u8,
        None => volume,
    };
    let adjustable = speaker_slot.is_some()
        && !app.soloist_waiting()
        && now.is_none_or(|now| now.can_set_volume);
    // The capsule grows leftward out of the speaker, 8 points past it.
    let capsule = Rect::from_min_size(
        pos2(
            speaker.right() + 8.0 - VOLUME.x,
            speaker.center().y - VOLUME.y / 2.0,
        ),
        VOLUME,
    );
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let now_s = ui.input(|input| input.time);
    let open_id = Id::new("mcloud-volume-open");
    let hover_start_id = Id::new("mcloud-volume-hover-start");
    let was_open = ui.data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    let on_speaker = speaker_slot.is_some() && pointer.is_some_and(|at| speaker.contains(at));
    let speaker_ready = if on_speaker {
        if was_open {
            true
        } else {
            let start = ui.data(|data| data.get_temp::<f64>(hover_start_id));
            match start {
                Some(start) => {
                    let elapsed = now_s - start;
                    if elapsed >= VOLUME_HOVER_DELAY {
                        true
                    } else {
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_secs_f64(
                                (VOLUME_HOVER_DELAY - elapsed).max(0.01),
                            ));
                        false
                    }
                }
                None => {
                    ui.data_mut(|data| data.insert_temp(hover_start_id, now_s));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs_f64(
                            VOLUME_HOVER_DELAY,
                        ));
                    false
                }
            }
        }
    } else {
        ui.data_mut(|data| data.remove::<f64>(hover_start_id));
        false
    };
    let hovering =
        speaker_ready || (was_open && pointer.is_some_and(|at| capsule.expand(4.0).contains(at)));
    let open = adjustable && (hovering || app.volume_preview.is_some());
    ui.data_mut(|data| data.insert_temp(open_id, open));
    let reveal = ui.ctx().animate_bool_with_time(open_id, open, 0.35);

    // The buttons beside the speaker fade as the capsule covers them.
    let faded = |color: Color32| color.gamma_multiply(1.0 - reveal);
    if reveal < 1.0 {
        let enabled = !open;
        if let Some(mini_slot) = slot("mini") {
            let mini = app.settings.winamp_window;
            let response = button(
                ui,
                mini_slot,
                Icon::McloudPopup,
                ICON,
                faded(if mini { palette.accent } else { normal() }),
                &gettext(locale, "Mini player"),
            );
            if response.clicked() && enabled {
                app.actions.push(Action::ToggleWinampWindow);
            }
        }

        if let Some(devices_slot) = slot("devices") {
            let remote = now.is_some_and(|now| !now.local) && !app.targets_own_soloist();
            let devices_open = remote || app.show_devices;
            let response = button(
                ui,
                devices_slot,
                Icon::McloudDevices,
                ICON,
                faded(if devices_open {
                    palette.accent
                } else {
                    normal()
                }),
                &gettext(locale, "Connect to a device"),
            );
            ui.ctx().data_mut(|data| {
                data.insert_temp(Id::new(crate::ui::devices::BUTTON_RECT_ID), response.rect);
            });
            if response.clicked() && enabled {
                app.actions.push(Action::ToggleDevicesPopup);
            }
        }

        if let Some(lyrics_slot) = slot("lyrics") {
            let full = app.lyrics_fullscreen.is_some();
            let response = button(
                ui,
                lyrics_slot,
                Icon::McloudLyrics,
                ICON,
                faded(if full { palette.accent } else { normal() }),
                &gettext(locale, "Full screen lyrics"),
            );
            if response.clicked() && enabled {
                app.actions.push(Action::SetLyricsFullscreen(!full));
            }
        }

        if let Some(pane_slot) = slot("nowplaying") {
            let open = app.show_lyrics_panel;
            let response = button(
                ui,
                pane_slot,
                Icon::McloudNowPlaying,
                ICON,
                faded(if open { palette.accent } else { normal() }),
                &gettext(locale, "Now Playing"),
            );
            if response.clicked() && enabled {
                app.actions.push(Action::ToggleLyricsPanel);
            }
        }

        if let Some(queue_slot) = slot("queue") {
            let queue_open = app.show_queue_panel || matches!(app.page(), Page::Queue);
            let response = button(
                ui,
                queue_slot,
                Icon::McloudQueue,
                ICON,
                faded(if queue_open { palette.accent } else { normal() }),
                &gettext(locale, "Queue"),
            );
            if response.clicked() && enabled {
                app.actions.push(Action::ToggleQueuePanel);
            }
        }
    }

    if reveal > 0.0 {
        volume_capsule(app, ui, capsule, reveal, shown, now);
    }

    if speaker_slot.is_none() {
        return left;
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
    let response = button(ui, speaker, icon, ICON, normal(), &label);
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
        crate::ui::mcloud::ink((26.0 * reveal) as u8),
    );
    painter.rect_stroke(
        visible,
        corner,
        Stroke::new(1.0, crate::ui::mcloud::ink((31.0 * reveal) as u8)),
        StrokeKind::Inside,
    );
    painter.hline(
        (visible.left() + VOLUME.y / 2.0)..=(visible.right() - VOLUME.y / 2.0),
        visible.top() + 1.0,
        Stroke::new(1.0, crate::ui::mcloud::ink((71.0 * reveal) as u8)),
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
        crate::ui::mcloud::ink((77.0 * reveal) as u8),
    );
    let mut filled = track;
    filled.set_right(track.left() + track.width() * level);
    painter.rect_filled(
        filled,
        track_corner,
        super::tone(Color32::WHITE, Color32::from_rgb(0x33, 0x33, 0x33)).gamma_multiply(reveal),
    );
}

/// Words over the glass carry mcloud's soft shadow (0 1px 6px #0008),
/// approximated by a faint copy a point lower.
fn shadowed(ui: &egui::Ui, pos: Pos2, galley: std::sync::Arc<egui::Galley>, color: Color32) {
    if super::is_dark() {
        ui.painter().galley(
            pos + vec2(0.0, 1.0),
            galley.clone(),
            Color32::from_black_alpha(90),
        );
    }
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

    let artwork = app.settings.mcloud_player_artwork;
    let side = if artwork { inner.height() } else { 0.0 };
    let art = Rect::from_min_size(inner.min, Vec2::splat(side));
    let art_response = ui.interact(art, Id::new("mcloud-player-art"), Sense::click_and_drag());
    if artwork {
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
    }
    if artwork && fade < 1.0 {
        ui.painter().rect_filled(
            art,
            CornerRadius::same(5),
            Color32::from_black_alpha((255.0 * (1.0 - fade) * 0.8) as u8),
        );
    }
    if artwork && art_response.hovered() && scrubbing == 0.0 {
        ui.painter()
            .rect_filled(art, CornerRadius::same(5), Color32::from_black_alpha(140));
        theme::paint_icon(ui, Icon::McloudExpand, art, side * 0.55, Color32::WHITE);
    }
    let art_dragged = art_response.drag_started_by(egui::PointerButton::Primary);
    // The song's menu, over the artwork and the words.
    let art_menu = art_response.clone();
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
            super::tone(
                Color32::from_rgb(0xe0, 0xe0, 0xe0),
                Color32::from_rgb(0x33, 0x33, 0x33),
            )
            .gamma_multiply(fade),
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
    let title_color = super::tone(
        Color32::from_rgb(0xf5, 0xf5, 0xf5),
        Color32::from_rgb(0x1d, 0x1d, 0x1f),
    )
    .gamma_multiply(fade);
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
        .interact(
            title_rect,
            Id::new("mcloud-player-title"),
            Sense::click_and_drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // The artwork and the title drag the playing song to a playlist or the queue.
    if (art_dragged || title_response.drag_started_by(egui::PointerButton::Primary))
        && let Some(item) = app.now_playing_item()
    {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            crate::model::DragTrack {
                title: item.name().to_string(),
                image: app.item_image(&item, 64),
                items: vec![item],
                from: None,
                source_playlist: None,
            },
        );
    }
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
                super::tone(
                    Color32::from_rgb(0x9a, 0x9a, 0x9a),
                    Color32::from_rgb(0x8e, 0x8e, 0x93),
                )
            };
            let star_icon = if saved {
                Icon::McloudStarFilled
            } else {
                Icon::McloudStar
            };
            theme::paint_icon(ui, star_icon, star, 13.0, color.gamma_multiply(fade));
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
            Sense::click_and_drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    marquee(
        ui,
        &subtitle(now),
        theme::regular(13.0),
        subtitle_rect,
        crate::ui::mcloud::ink(208).gamma_multiply(fade),
        ui.rect_contains_pointer(lcd),
    );
    // The line under the title drags the album it names.
    if subtitle_response.drag_started_by(egui::PointerButton::Primary)
        && let Some(id) = &now.album_id
    {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            crate::model::DragAlbum {
                uri: format!("spotify:album:{id}"),
                title: now.album_name.clone(),
            },
        );
    }
    // Spotify's way: each name is a link, while the whole line can be seen.
    let line = subtitle(now);
    let line_fits = ui
        .painter()
        .layout_no_wrap(line.clone(), theme::regular(13.0), Color32::WHITE)
        .size()
        .x
        <= subtitle_rect.width();
    let links = app.settings.mcloud_lcd_links && line_fits;
    if links
        && subtitle_response.clicked()
        && let Some(at) = subtitle_response.interact_pointer_pos()
    {
        let mut left = subtitle_rect.left();
        let mut target = None;
        for (index, artist) in now.artists.iter().enumerate() {
            let name = if index + 1 < now.artists.len() {
                format!("{}, ", artist.name)
            } else {
                artist.name.clone()
            };
            let width = ui
                .painter()
                .layout_no_wrap(name, theme::regular(13.0), Color32::WHITE)
                .size()
                .x;
            if at.x < left + width {
                target = artist.id.clone().map(Page::Artist);
                break;
            }
            left += width;
        }
        let page = target
            .or_else(|| now.album_id.clone().map(Page::Album))
            .or_else(|| now.show_id.clone().map(Page::Show));
        if let Some(page) = page {
            app.actions.push(Action::Open(page));
        }
    }
    if let Some(item) = app.now_playing_item() {
        let context = app.editable_context_playlist();
        for response in [&art_menu, &title_response, &subtitle_response] {
            egui::Popup::context_menu(response)
                .id(response.id.with("song-menu"))
                .frame(widgets::menu_frame(palette))
                .show(|ui| widgets::item_menu(ui, app, &item, context.as_ref(), None));
        }
    }
    let artists_with_id: Vec<_> = now.artists.iter().filter(|a| a.id.is_some()).collect();
    let album_id = now.album_id.clone().filter(|id| !id.is_empty());
    if !links && !artists_with_id.is_empty() && album_id.is_some() {
        egui::Popup::menu(&subtitle_response)
            .frame(widgets::menu_frame(palette))
            .align(egui::RectAlign::TOP_START)
            .show(|ui| {
                ui.set_min_width(200.0);
                if artists_with_id.len() == 1 {
                    if widgets::menu_item(
                        ui,
                        palette,
                        Some(Icon::User),
                        &gettext(locale, "Go to Artist"),
                    ) && let Some(id) = artists_with_id[0].id.clone()
                    {
                        app.actions.push(Action::Open(Page::Artist(id)));
                    }
                } else {
                    let label = gettext(locale, "Go to Artist");
                    widgets::menu_submenu(ui, palette, Some(Icon::User), &label, |ui| {
                        ui.set_min_width(200.0);
                        for artist in &artists_with_id {
                            if widgets::menu_item(ui, palette, Some(Icon::User), &artist.name)
                                && let Some(id) = artist.id.clone()
                            {
                                app.actions.push(Action::Open(Page::Artist(id)));
                            }
                        }
                    });
                }
                if let Some(album_id) = &album_id
                    && widgets::menu_item(
                        ui,
                        palette,
                        Some(Icon::Disc),
                        &gettext(locale, "Go to Album"),
                    )
                {
                    app.actions
                        .push(Action::Open(Page::Album(album_id.clone())));
                }
            });
    } else if subtitle_response.clicked() {
        if let Some(id) = artists_with_id.first().and_then(|artist| artist.id.clone()) {
            app.actions.push(Action::Open(Page::Artist(id)));
        } else if let Some(album_id) = album_id {
            app.actions.push(Action::Open(Page::Album(album_id)));
        }
    }

    let (position, duration) = (now.position_ms, now.duration_ms);
    let shown_position = app
        .seek_preview
        .map_or(position, |fraction| (fraction * duration as f32) as u32);
    if scrubbing > 0.0 {
        let color = super::tone(
            Color32::from_rgb(0xd4, 0xd4, 0xd4),
            Color32::from_rgb(0x3d, 0x3d, 0x3d),
        )
        .gamma_multiply(scrubbing);
        let y = lcd.bottom() - 19.0 - 8.0;
        rolling_time(
            ui,
            Id::new("mcloud-time-elapsed"),
            &util::format_duration_ms(shown_position),
            lcd.left() + 1.0,
            false,
            y,
            color,
        );
        rolling_time(
            ui,
            Id::new("mcloud-time-remaining"),
            &format!(
                "-{}",
                util::format_duration_ms(duration.saturating_sub(shown_position))
            ),
            lcd.right() - 1.0,
            true,
            y,
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

/// How far along its 0.32s a changing digit is, and how far it has moved
/// (mcloud's `cubic-bezier(.2,.8,.2,1)`, approximated by an ease-out).
fn digit_progress(elapsed: f64) -> f32 {
    let t = (elapsed / 0.32).clamp(0.0, 1.0) as f32;
    1.0 - (1.0 - t).powi(3)
}

/// The digits that changed between two times, with which way they roll: up as
/// time grows, down when it shrinks.
fn changed_digits(old: &str, new: &str) -> Vec<bool> {
    let (old_len, new_len) = (old.chars().count(), new.chars().count());
    new.chars()
        .enumerate()
        .map(|(index, ch)| {
            // Aligned at the end, so 9:59 to 10:00 keeps its columns.
            let back = new_len - index;
            old_len < back || old.chars().nth(old_len - back) != Some(ch)
        })
        .collect()
}

/// A time whose changing digits roll: the new one rises into place while the
/// old one leaves, as in Apple Music.
fn rolling_time(
    ui: &egui::Ui,
    id: Id,
    text: &str,
    x: f32,
    from_right: bool,
    y: f32,
    color: Color32,
) {
    #[derive(Clone)]
    struct Roll {
        shown: String,
        before: String,
        changed_at: f64,
    }
    let now = ui.input(|input| input.time);
    let mut roll = ui
        .data(|data| data.get_temp::<Roll>(id))
        .unwrap_or_else(|| Roll {
            shown: text.to_string(),
            before: text.to_string(),
            changed_at: f64::NEG_INFINITY,
        });
    if roll.shown != text {
        roll.before = std::mem::replace(&mut roll.shown, text.to_string());
        roll.changed_at = now;
    }
    let elapsed = now - roll.changed_at;
    let progress = if crate::motion::reduced(ui.ctx()) {
        1.0
    } else {
        digit_progress(elapsed)
    };
    if progress < 1.0 {
        ui.ctx().request_repaint();
    }
    let font = theme::semibold(13.0);
    let digit_width = |ch: char| {
        ui.painter()
            .layout_no_wrap(ch.to_string(), font.clone(), color)
            .size()
            .x
    };
    let widths: Vec<f32> = text.chars().map(digit_width).collect();
    let total: f32 = widths.iter().sum();
    let mut left = if from_right { x - total } else { x };
    let height = ui
        .painter()
        .layout_no_wrap("0".into(), font.clone(), color)
        .size()
        .y;
    let changed = changed_digits(&roll.before, text);
    let (old_len, new_len) = (roll.before.chars().count(), text.chars().count());
    // Elapsed grows and remaining shrinks: a down-counting time rolls down.
    let direction = if from_right { -1.0 } else { 1.0 };
    for (index, (ch, width)) in text.chars().zip(widths).enumerate() {
        let center = pos2(left + width / 2.0, y);
        if changed[index] && progress < 1.0 {
            let shift = height * 0.6 * direction;
            ui.painter().text(
                center + vec2(0.0, shift * (1.0 - progress)),
                Align2::CENTER_CENTER,
                ch,
                font.clone(),
                color.gamma_multiply(progress),
            );
            let back = new_len - index;
            if old_len >= back
                && let Some(old) = roll.before.chars().nth(old_len - back)
            {
                ui.painter().text(
                    center - vec2(0.0, shift * progress),
                    Align2::CENTER_CENTER,
                    old,
                    font.clone(),
                    color.gamma_multiply(1.0 - progress),
                );
            }
        } else {
            ui.painter()
                .text(center, Align2::CENTER_CENTER, ch, font.clone(), color);
        }
        left += width;
    }
    ui.data_mut(|data| data.insert_temp(id, roll));
}

/// Text that fits stands still; text that does not scrolls while the
/// pointer is on the LCD and otherwise ends with an ellipsis, so an idle
/// window does not repaint for it.
pub(in crate::ui) fn marquee(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    rect: Rect,
    color: Color32,
    moving: bool,
) {
    const GAP: f32 = 48.0;
    const SPEED: f32 = 28.0;
    const PAUSE: f64 = 1.2;
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let left = |galley: &std::sync::Arc<egui::Galley>| {
        pos2(rect.left(), rect.center().y - galley.size().y / 2.0)
    };
    let start_id = Id::new("mcloud-marquee-start");
    let active = ui.input(|input| input.focused && !input.viewport().minimized.unwrap_or(false));
    if full.size().x <= rect.width() || !moving || !active {
        let galley = widgets::ellipsized(ui, text, font, color, rect.width(), 1);
        shadowed(ui, left(&galley), galley, color);
        ui.ctx().data_mut(|data| data.remove::<f64>(start_id));
        return;
    }
    let now = ui.input(|input| input.time);
    let start = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(start_id, || now));
    let since_start = (now - start).max(0.0);
    if since_start < PAUSE {
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.galley(left(&full), full, color);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(
                (PAUSE - since_start).max(0.016),
            ));
        return;
    }
    let elapsed = (since_start - PAUSE) as f32;
    let cycle = full.size().x + GAP;
    let ppp = ui.ctx().pixels_per_point().max(1.0);
    let offset = (((elapsed * SPEED) % cycle) * ppp).round() / ppp;
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let origin = left(&full) - vec2(offset, 0.0);
    painter.galley(origin, full.clone(), color);
    painter.galley(origin + vec2(cycle, 0.0), full, color);
    let step_s = (1.0 / (ppp * SPEED)).clamp(0.016, 0.05);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f32(step_s));
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
    painter.rect_filled(track, corner, crate::ui::mcloud::ink(51));
    let ppp = ui.ctx().pixels_per_point().max(1.0);
    let mut filled = track;
    filled.set_right(track.left() + (track.width() * fraction * ppp).round() / ppp);
    painter.rect_filled(
        filled,
        corner,
        super::tone(
            Color32::from_rgb(0xe9, 0xe9, 0xe9),
            Color32::from_rgb(0x3d, 0x3d, 0x3d),
        ),
    );
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
    #[test]
    fn the_eco_capsules_gradient_runs_through_its_round_ends() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), vec2(300.0, 54.0));
        let (top, bottom) = (Color32::from_gray(60), Color32::from_gray(40));
        let mesh = super::capsule_gradient(rect, top, bottom);
        // Every pair of vertices is one row: both ends have that row's colour,
        // so the ends can't differ from the middle.
        for row in mesh.vertices.as_chunks::<2>().0 {
            assert_eq!(row[0].color, row[1].color);
            assert_eq!(row[0].pos.y, row[1].pos.y);
        }
        let middle = &mesh.vertices[mesh.vertices.len() / 2 - 1];
        assert!(
            middle.pos.x < rect.left() + 1.0,
            "the widest row reaches the end"
        );
        assert_eq!(mesh.vertices[0].color.r(), 60);
        assert!(
            mesh.vertices
                .last()
                .is_some_and(|last| last.color.r() <= 41)
        );
        // Nothing leaves the capsule, whose top row is only its straight part.
        assert!(mesh.vertices[0].pos.x > rect.left() + 20.0);
    }

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

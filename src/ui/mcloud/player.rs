//! The floating player, after Apple Music's: a glass capsule over the page
//! that grows with the window up to [`MAX_WIDTH`], then stays centred.
//! Transport on the left; in the middle the "LCD" with the artwork, the
//! title, "Artist — Album" and a hairline scrubber along its bottom; device,
//! lyrics, queue and volume on the right, where the volume slider grows
//! out of the speaker over the buttons beside it.

use egui::{
    Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder,
    Vec2, pos2, vec2,
};

use crate::app::{App, NowPlaying};
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::player::RepeatMode;
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets::{self, SliderEvent};
use crate::util;

pub const HEIGHT: f32 = 52.0;
const RADIUS: f32 = HEIGHT / 2.0;
const MAX_WIDTH: f32 = 700.0;
const MIN_WIDTH: f32 = 460.0;
/// The gap between the capsule and the window's bottom edge.
pub const BOTTOM: f32 = 14.0;
const PADDING: f32 = 14.0;
const BUTTON: f32 = 30.0;
const ART: f32 = 36.0;
const VOLUME_SLIDER: f32 = 92.0;

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
            // Room for the shadow, which spreads past the capsule.
            ui.set_clip_rect(rect.expand(40.0));
            // The capsule is the area's own: clicks between its controls
            // stay on it rather than reaching the page beneath.
            ui.allocate_exact_size(rect.size(), Sense::click());
            let palette = app.palette;
            surface(ui, &palette, rect, app.settings.mcloud_glass);
            let now = app.now_playing();
            let transport_right = transport(app, ui, rect, now.as_ref());
            let extras_left = extras(app, ui, rect, now.as_ref());
            let lcd = Rect::from_min_max(
                pos2(transport_right + 10.0, rect.top()),
                pos2(extras_left - 10.0, rect.bottom()),
            );
            if lcd.width() > ART + 40.0 {
                display(app, ui, lcd, now.as_ref());
            }
        });
}

/// Where the capsule sits over `content`.
fn capsule(content: Rect) -> Rect {
    let room = content.width() - 32.0;
    let width = room.clamp(MIN_WIDTH.min(room.max(0.0)), MAX_WIDTH);
    // Whole points, so the glass never lands on half a pixel and blurs
    // its own edge.
    let center = pos2(
        content.center().x.round(),
        (content.bottom() - BOTTOM - HEIGHT / 2.0).round(),
    );
    Rect::from_center_size(center, vec2(width.round(), HEIGHT))
}

/// The capsule itself: its shadow, the glass, a tint over it, and the
/// hairlines that catch the light along its edge.
fn surface(ui: &egui::Ui, palette: &Palette, rect: Rect, effect: crate::settings::GlassEffect) {
    let corner = CornerRadius::same(RADIUS as u8);
    let painter = ui.painter();
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(rect, corner),
    );
    let glass = super::glass::paint(ui, rect, RADIUS, effect);
    // The glass shows the page through the panel's colour; a solid
    // surface is the overlay's, a step above the page.
    let tint = if glass {
        palette.panel.gamma_multiply(0.42)
    } else {
        palette.overlay
    };
    painter.rect_filled(rect, corner, tint);
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, Color32::from_white_alpha(28)),
        StrokeKind::Inside,
    );
    // Light along the top edge, as on Apple's glass.
    painter.hline(
        (rect.left() + RADIUS)..=(rect.right() - RADIUS),
        rect.top() + 1.5,
        Stroke::new(1.0, Color32::from_white_alpha(22)),
    );
}

fn button_rect(center_x: f32, center_y: f32) -> Rect {
    Rect::from_center_size(pos2(center_x, center_y), Vec2::splat(BUTTON))
}

/// An icon button centred in `rect`.
fn icon_button(
    ui: &mut egui::Ui,
    rect: Rect,
    icon: Icon,
    size: f32,
    colors: (Color32, Color32),
    tooltip: &str,
) -> egui::Response {
    let mut cell = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    theme::icon_button(&mut cell, icon, size, colors.0, colors.1, tooltip)
}

/// Shuffle, previous, play, next and repeat. Returns their right edge.
fn transport(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let cy = rect.center().y;
    let enabled = now.is_some_and(|now| now.can_control) || app.is_connected();
    let playing = now.is_some_and(|now| now.playing);
    let loading = now.is_some_and(|now| now.loading) || app.any_play_pending();
    let shuffle = now.map_or_else(|| app.playing_context_shuffle(), |now| now.shuffle);
    let repeat = now.map(|now| now.repeat).unwrap_or_default();
    let main = if enabled { palette.text } else { palette.dim };
    let quiet = if enabled {
        palette.secondary
    } else {
        palette.dim
    };
    let mut x = rect.left() + PADDING + BUTTON / 2.0;
    let mut next_slot = || {
        let slot = button_rect(x, cy);
        x += BUTTON;
        slot
    };

    let shuffle_slot = next_slot();
    let (color, hover) = if shuffle {
        (palette.accent, palette.accent_hover)
    } else {
        (quiet, palette.text)
    };
    let response = icon_button(
        ui,
        shuffle_slot,
        Icon::Shuffle,
        15.0,
        (color, hover),
        &gettext(locale, "Shuffle"),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            true,
            shuffle,
            gettext(locale, "Shuffle"),
        )
    });
    if response.clicked() {
        app.actions.push(Action::ToggleShuffle);
    }

    if icon_button(
        ui,
        next_slot(),
        Icon::SkipBackFilled,
        17.0,
        (main, palette.text),
        &gettext(locale, "Previous"),
    )
    .clicked()
    {
        app.actions.push(Action::Previous);
    }

    let play_slot = next_slot();
    if loading {
        let mut cell = ui.new_child(
            UiBuilder::new()
                .max_rect(play_slot)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        theme::spinner(&mut cell, 18.0, palette.text);
    } else {
        let (icon, label) = if playing {
            (Icon::PauseFilled, gettext(locale, "Pause"))
        } else {
            (Icon::PlayFilled, gettext(locale, "Play"))
        };
        if icon_button(ui, play_slot, icon, 22.0, (main, palette.text), &label).clicked() {
            app.actions.push(Action::TogglePlay);
        }
    }

    if icon_button(
        ui,
        next_slot(),
        Icon::SkipForwardFilled,
        17.0,
        (main, palette.text),
        &gettext(locale, "Next"),
    )
    .clicked()
    {
        app.actions.push(Action::Next);
    }

    let (icon, color, tooltip) = match repeat {
        RepeatMode::Off => (Icon::Repeat, quiet, gettext(locale, "Repeat")),
        RepeatMode::Context => (Icon::Repeat, palette.accent, gettext(locale, "Repeat one")),
        RepeatMode::Track => (Icon::Repeat1, palette.accent, gettext(locale, "Repeat off")),
    };
    let hover = if repeat == RepeatMode::Off {
        palette.text
    } else {
        palette.accent_hover
    };
    let repeat_slot = next_slot();
    if icon_button(ui, repeat_slot, icon, 15.0, (color, hover), &tooltip).clicked() {
        app.actions.push(Action::CycleRepeat);
    }
    repeat_slot.right()
}

/// Devices, lyrics, queue and volume, right to left from the capsule's
/// end. While the pointer is on the speaker or the slider, the slider
/// covers the buttons beside it. Returns the group's left edge.
fn extras(app: &mut App, ui: &mut egui::Ui, rect: Rect, now: Option<&NowPlaying>) -> f32 {
    let palette = app.palette;
    let locale = app.locale;
    let cy = rect.center().y;
    let speaker = button_rect(rect.right() - PADDING - BUTTON / 2.0, cy);
    let queue = speaker.translate(vec2(-BUTTON, 0.0));
    let lyrics = queue.translate(vec2(-BUTTON, 0.0));
    let devices = lyrics.translate(vec2(-BUTTON, 0.0));

    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown = match app.volume_preview {
        Some(fraction) => (fraction * 100.0).round() as u8,
        None => volume,
    };
    let adjustable = !app.soloist_waiting() && now.is_none_or(|now| now.can_set_volume);
    let slider_rect = Rect::from_min_max(
        pos2(speaker.left() - VOLUME_SLIDER - 4.0, speaker.top()),
        pos2(speaker.left(), speaker.bottom()),
    );
    // Open while the pointer is on the speaker or the slider, or a drag holds it.
    let open_id = Id::new("mcloud-volume-open");
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let hovering = pointer.is_some_and(|at| speaker.union(slider_rect).contains(at));
    let open = adjustable && (hovering || app.volume_preview.is_some());
    let reveal = ui.ctx().animate_bool_with_time(open_id, open, 0.18);

    // The buttons the slider covers fade as it grows.
    let covered = |rect: Rect| rect.left() >= slider_rect.left() - 1.0;
    let fade = |color: Color32, rect: Rect| {
        if covered(rect) {
            color.gamma_multiply(1.0 - reveal)
        } else {
            color
        }
    };

    if reveal < 1.0 {
        let remote = now.is_some_and(|now| !now.local) && !app.targets_own_soloist();
        let icon = now
            .filter(|_| remote)
            .and_then(|now| now.device_kind.as_deref())
            .map_or(Icon::Speaker, crate::ui::devices::device_icon);
        let color = if remote {
            palette.accent
        } else {
            palette.secondary
        };
        let response = icon_button(
            ui,
            devices,
            icon,
            16.0,
            (fade(color, devices), fade(palette.text, devices)),
            &gettext(locale, "Connect to a device"),
        );
        ui.ctx().data_mut(|data| {
            data.insert_temp(Id::new(crate::ui::devices::BUTTON_RECT_ID), response.rect)
        });
        if response.clicked() && !open {
            app.actions.push(Action::ToggleDevicesPopup);
        }

        let lyrics_open = app.show_lyrics_panel || app.lyrics_fullscreen.is_some();
        let color = if lyrics_open {
            palette.accent
        } else {
            palette.secondary
        };
        if icon_button(
            ui,
            lyrics,
            Icon::Mic,
            16.0,
            (fade(color, lyrics), fade(palette.text, lyrics)),
            &gettext(locale, "Lyrics"),
        )
        .clicked()
            && !open
        {
            app.actions.push(Action::ToggleLyricsPanel);
        }

        let queue_open = app.show_queue_panel || matches!(app.page(), Page::Queue);
        let color = if queue_open {
            palette.accent
        } else {
            palette.secondary
        };
        if icon_button(
            ui,
            queue,
            Icon::ListVideo,
            16.0,
            (fade(color, queue), fade(palette.text, queue)),
            &gettext(locale, "Queue"),
        )
        .clicked()
            && !open
        {
            app.actions.push(Action::ToggleQueuePanel);
        }
    }

    if reveal > 0.0 {
        // The slider grows leftward out of the speaker.
        let grown = Rect::from_min_max(
            pos2(
                slider_rect.right() - slider_rect.width() * reveal,
                slider_rect.top(),
            ),
            slider_rect.max,
        );
        let mut slider_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(grown.shrink2(vec2(4.0, 7.0)))
                .layout(Layout::left_to_right(egui::Align::Center)),
        );
        slider_ui.set_clip_rect(grown);
        slider_ui.add_enabled_ui(adjustable, |ui| {
            match widgets::thin_slider(
                ui,
                &palette,
                Id::new("mcloud-volume-slider"),
                &gettext(locale, "Volume (%)"),
                shown as f32 / 100.0,
                VOLUME_SLIDER - 8.0,
                Some(0.05),
            ) {
                SliderEvent::Dragging(value) => {
                    app.volume_preview = Some(value);
                    if now.is_none_or(|now| now.local) {
                        app.actions
                            .push(Action::PreviewVolume((value * 100.0).round() as u8));
                    }
                }
                SliderEvent::Committed(value) => {
                    app.volume_preview = None;
                    app.actions
                        .push(Action::SetVolume((value * 100.0).round() as u8));
                }
                SliderEvent::None => {}
            }
        });
    }

    let icon = match shown {
        0 => Icon::VolumeX,
        1..=33 => Icon::Volume,
        34..=66 => Icon::Volume1,
        _ => Icon::Volume2,
    };
    let label = if shown == 0 {
        gettext(locale, "Unmute")
    } else {
        gettext(locale, "Mute")
    };
    let response = icon_button(
        ui,
        speaker,
        icon,
        17.0,
        (palette.secondary, palette.text),
        &label,
    );
    if adjustable {
        // The wheel over the speaker steps the volume, as over the slider.
        let notches = widgets::wheel_notches(ui, &response);
        if notches != 0 {
            let volume = (i32::from(shown) + notches * 5).clamp(0, 100) as u8;
            app.actions.push(Action::SetVolume(volume));
        }
        if response.clicked() {
            app.actions.push(Action::ToggleMute);
        }
    }
    devices.left()
}

/// The LCD: artwork, title, the scrolling "Artist — Album" line, the heart
/// and the "…" menu, and the hairline scrubber along its bottom.
fn display(app: &mut App, ui: &mut egui::Ui, lcd: Rect, now: Option<&NowPlaying>) {
    let palette = app.palette;
    let locale = app.locale;
    let cy = lcd.center().y;
    let art = Rect::from_min_size(pos2(lcd.left(), cy - ART / 2.0), Vec2::splat(ART));
    let Some(now) = now else {
        widgets::paint_cover(ui, &palette, None, art, 6.0, Icon::Music, None);
        let galley = widgets::ellipsized(
            ui,
            &gettext(locale, "Pick a song, album, or playlist"),
            theme::regular(12.5),
            palette.secondary,
            lcd.right() - art.right() - 10.0,
            1,
        );
        ui.painter().galley(
            pos2(art.right() + 10.0, cy - galley.size().y / 2.0),
            galley,
            palette.secondary,
        );
        return;
    };

    // Artwork; hovering it offers full-screen lyrics, as mcloud's does.
    let art_response = ui.interact(art, Id::new("mcloud-player-art"), Sense::click());
    widgets::paint_cover(
        ui,
        &palette,
        now.art_small.as_deref().or(now.art_url.as_deref()),
        art,
        6.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    if art_response.hovered() {
        ui.painter()
            .rect_filled(art, CornerRadius::same(6), Color32::from_black_alpha(120));
        theme::paint_icon(ui, Icon::Expand, art, 16.0, Color32::WHITE);
    }
    let art_response = art_response.on_hover_text(gettext(locale, "Full screen lyrics"));
    if art_response.clicked() {
        app.actions.push(Action::SetLyricsFullscreen(true));
    }

    // Heart and "…" at the LCD's end.
    let more = button_rect(lcd.right() - BUTTON / 2.0, cy - 5.0);
    let heart = more.translate(vec2(-BUTTON + 4.0, 0.0));
    let song = app.now_playing_item();
    if let Some(item) = &song {
        let response = icon_button(
            ui,
            more,
            Icon::Ellipsis,
            16.0,
            (palette.secondary, palette.text),
            &gettext(locale, "More"),
        );
        let context = app.editable_context_playlist();
        egui::Popup::menu(&response)
            .frame(widgets::menu_frame(&palette))
            .align(egui::RectAlign::TOP_END)
            .show(|ui| widgets::item_menu(ui, app, item, context.as_ref(), None));
    }
    let text_right = if now.is_episode {
        more.left()
    } else {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        let (icon, color, hover, tooltip) = if saved {
            (
                Icon::HeartFilled,
                palette.accent,
                palette.accent_hover,
                gettext(locale, "Remove from Liked Songs"),
            )
        } else {
            (
                Icon::Heart,
                palette.secondary,
                palette.text,
                gettext(locale, "Save to Liked Songs"),
            )
        };
        if icon_button(ui, heart, icon, 15.0, (color, hover), &tooltip).clicked() {
            app.actions.push(Action::ToggleSaved(now.uri.clone()));
        }
        heart.left()
    };

    let text_left = art.right() + 10.0;
    let text_width = (text_right - text_left).max(0.0);
    let title_y = cy - 9.0;
    let title = widgets::ellipsized(
        ui,
        &now.title,
        theme::semibold(13.0),
        palette.text,
        text_width,
        1,
    );
    let title_rect = Rect::from_min_size(
        pos2(text_left, title_y - title.size().y / 2.0),
        title.size(),
    );
    let title_response = ui
        .interact(title_rect, Id::new("mcloud-player-title"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    ui.painter().galley(title_rect.min, title, palette.text);
    if title_response.clicked() {
        if let Some(id) = &now.album_id {
            app.actions.push(Action::Open(Page::Album(id.clone())));
        } else if let Some(id) = &now.show_id {
            app.actions.push(Action::Open(Page::Show(id.clone())));
        }
    }

    // The scrubber's band: hovering it swaps the subtitle for the times.
    let bar_y = lcd.bottom() - 6.0;
    let scrub_band =
        Rect::from_min_max(pos2(text_left, bar_y - 6.0), pos2(lcd.right(), bar_y + 5.0));
    let scrub = ui.interact(
        scrub_band,
        Id::new("mcloud-player-scrubber"),
        Sense::click_and_drag(),
    );
    let lcd_hovered = ui.rect_contains_pointer(lcd);
    let scrubbing = scrub.hovered() || scrub.dragged();
    let show_times =
        ui.ctx()
            .animate_bool_with_time(Id::new("mcloud-player-times"), scrubbing, 0.15);

    let subtitle_y = cy + 8.0;
    let subtitle_text = subtitle(now);
    if show_times < 1.0 {
        marquee(
            ui,
            &subtitle_text,
            Rect::from_min_max(
                pos2(text_left, subtitle_y - 8.0),
                pos2(text_left + text_width, subtitle_y + 8.0),
            ),
            palette.secondary.gamma_multiply(1.0 - show_times),
            lcd_hovered,
        );
    }

    let (position, duration) = (now.position_ms, now.duration_ms);
    let shown_position = app
        .seek_preview
        .map_or(position, |fraction| (fraction * duration as f32) as u32);
    if show_times > 0.0 {
        let color = palette.secondary.gamma_multiply(show_times);
        ui.painter().text(
            pos2(text_left, subtitle_y),
            Align2::LEFT_CENTER,
            util::format_duration_ms(shown_position),
            theme::regular(10.5),
            color,
        );
        ui.painter().text(
            pos2(lcd.right(), subtitle_y),
            Align2::RIGHT_CENTER,
            format!(
                "-{}",
                util::format_duration_ms(duration.saturating_sub(shown_position))
            ),
            theme::regular(10.5),
            color,
        );
    }
    scrubber(
        app,
        ui,
        &scrub,
        scrub_band,
        bar_y,
        show_times,
        duration,
        shown_position,
    );
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
/// pointer is on the display and otherwise ends with an ellipsis, so an
/// idle window does not repaint for it.
fn marquee(ui: &egui::Ui, text: &str, rect: Rect, color: Color32, moving: bool) {
    const GAP: f32 = 36.0;
    const SPEED: f32 = 28.0;
    const PAUSE: f64 = 1.2;
    let font = theme::regular(12.0);
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let left = |galley: &std::sync::Arc<egui::Galley>| {
        pos2(rect.left(), rect.center().y - galley.size().y / 2.0)
    };
    if full.size().x <= rect.width() || !moving {
        let galley = widgets::ellipsized(ui, text, font, color, rect.width(), 1);
        ui.painter().galley(left(&galley), galley, color);
        ui.ctx()
            .data_mut(|data| data.remove::<f64>(Id::new("mcloud-marquee-start")));
        return;
    }
    let now = ui.input(|input| input.time);
    let start_id = Id::new("mcloud-marquee-start");
    let start = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(start_id, || now));
    let elapsed = (now - start - PAUSE).max(0.0) as f32;
    let cycle = full.size().x + GAP;
    let offset = (elapsed * SPEED) % cycle;
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let origin: Pos2 = left(&full) - vec2(offset, 0.0);
    painter.galley(origin, full.clone(), color);
    painter.galley(origin + vec2(cycle, 0.0), full, color);
    ui.ctx().request_repaint();
}

/// The hairline along the LCD's bottom: it thickens under the pointer, and
/// a click or a drag anywhere along it seeks.
#[allow(clippy::too_many_arguments)]
fn scrubber(
    app: &mut App,
    ui: &egui::Ui,
    response: &egui::Response,
    band: Rect,
    y: f32,
    emphasis: f32,
    duration: u32,
    position: u32,
) {
    let palette = app.palette;
    let fraction_at = |x: f32| ((x - band.left()) / band.width()).clamp(0.0, 1.0);
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
    let thickness = 2.0 + 2.0 * emphasis;
    let track = Rect::from_min_max(
        pos2(band.left(), y - thickness / 2.0),
        pos2(band.right(), y + thickness / 2.0),
    );
    let corner = CornerRadius::same(2);
    let painter = ui.painter();
    painter.rect_filled(track, corner, Color32::from_white_alpha(38));
    let mut filled = track;
    filled.set_right(track.left() + track.width() * fraction);
    painter.rect_filled(filled, corner, palette.text.gamma_multiply(0.85));
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
        let narrow = capsule(content(600.0));
        assert_eq!(narrow.width(), 568.0);
        let wide = capsule(content(1600.0));
        assert_eq!(wide.width(), MAX_WIDTH);
        assert_eq!(wide.center().x, content(1600.0).center().x);
        assert_eq!(wide.bottom(), 800.0 - BOTTOM);
        // Narrower than the controls need, it still fits the page.
        assert!(capsule(content(400.0)).width() <= 400.0);
    }
}

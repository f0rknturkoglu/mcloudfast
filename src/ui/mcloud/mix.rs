//! Spotify's mix covers as Apple draws its own (user.css `.am-mix`): the photo
//! behind Spotify's image with the name in a band of the photo's colour, as
//! Essentials; a radio's photo in a circle on that colour, as a station; a mix
//! with no photo, as a personal mix, the name large on a gradient.

use egui::{Color32, CornerRadius, Mesh, Rect, Vec2, pos2, vec2};

use crate::app::App;
use crate::theme;
use crate::ui::widgets;

/// Where a mix's photo comes from, read from its image's address.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MixSource {
    Photo(String),
    Artist(String),
    Track(String),
    /// Mood mixes, Discover Weekly and daylists have no photo.
    None,
}

/// The source of a mix's image, or `None` when the image is not a mix's.
pub(crate) fn mix_source(url: &str) -> Option<MixSource> {
    const PICKASSO: &str = "https://pickasso.spotifycdn.com/image/";
    if url.starts_with("https://seed-mix-image.spotifycdn.com/")
        || url.starts_with("https://daylist.spotifycdn.com/")
    {
        return Some(MixSource::None);
    }
    let path = url.strip_prefix(PICKASSO)?;
    let (_, path) = path.split_once("/dt/v1/img/")?;
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let parts: Vec<&str> = path.split('/').collect();
    if parts.first() == Some(&"daily")
        && let Some(hash) = parts.get(2)
        && hash.len() >= 24
        && hash.starts_with("ab67")
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Some(MixSource::Photo(format!("https://i.scdn.co/image/{hash}")));
    }
    let id = parts
        .iter()
        .find(|part| part.len() == 22 && part.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    Some(match id {
        Some(id) if parts.first() == Some(&"radio") && parts.get(1) == Some(&"track") => {
            MixSource::Track((*id).to_string())
        }
        Some(id) => MixSource::Artist((*id).to_string()),
        None => MixSource::None,
    })
}

/// How a mix is drawn: Apple's pairing of radios with stations and of the
/// other mixes with Essentials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MixStyle {
    Band,
    Station,
    Gradient,
}

/// Everything a mix's cover needs once it is known.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MixCover {
    pub style: MixStyle,
    pub photo: Option<String>,
    pub band: Color32,
    pub from: Color32,
    pub to: Color32,
}

fn hsl_of(rgb: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = rgb.map(|channel| f32::from(channel) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let (delta, lightness) = (max - min, (max + min) / 2.0);
    if delta <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs()).max(1e-6);
    let hue = if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (hue * 60.0, saturation, lightness)
}

fn colour_of(hue: f32, saturation: f32, lightness: f32) -> Color32 {
    let a = saturation * lightness.min(1.0 - lightness);
    let channel = |n: f32| {
        let k = (n + hue / 30.0).rem_euclid(12.0);
        let value = lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (value * 255.0).round() as u8
    };
    Color32::from_rgb(channel(0.0), channel(8.0), channel(4.0))
}

/// The cover's colours from the photo's (theme.js `drawMix`): the band light
/// and soft for black text, the gradient two stops 30 degrees apart.
pub(crate) fn mix_colours(rgb: [u8; 3]) -> (Color32, Color32, Color32) {
    let (hue, saturation, lightness) = hsl_of(rgb);
    let hue = hue.round();
    let band = colour_of(
        hue,
        saturation.min(0.55),
        (lightness + 0.25).clamp(0.6, 0.78),
    );
    let chroma = saturation.min(0.7);
    let middle = lightness.clamp(0.4, 0.6);
    let from = colour_of(hue, chroma, middle + 0.12);
    let to = colour_of(
        (hue + 330.0) % 360.0,
        (chroma + 0.1).min(0.9),
        middle - 0.14,
    );
    (band, from, to)
}

/// Draws a mix's cover over `rect`. False while it is not known yet or the
/// image is not a mix's, when the caller draws Spotify's image.
pub(in crate::ui) fn paint(
    app: &mut App,
    ui: &egui::Ui,
    url: Option<&str>,
    rect: Rect,
    radius: f32,
    name: &str,
) -> bool {
    if !app.settings.mcloud_mix_covers {
        return false;
    }
    let Some(cover) = url.and_then(|url| app.mix_cover(url)) else {
        return false;
    };
    let round = CornerRadius::same(radius.min(127.0) as u8);
    let width = rect.width();
    match (cover.style, cover.photo.as_deref()) {
        (MixStyle::Band, Some(photo)) => {
            ui.painter().rect_filled(rect, round, cover.band);
            widgets::paint_cover(
                ui,
                &app.palette,
                Some(photo),
                rect,
                radius,
                theme::Icon::Music,
                Some(app.backend.art()),
            );
            let text_width = width * (1.0 - 2.0 * 0.06);
            let text_colour = Color32::from_rgba_unmultiplied(0, 0, 0, 0xe0);
            let title = widgets::ellipsized(
                ui,
                name,
                theme::medium(width * 0.12),
                text_colour,
                text_width,
                2,
            );
            let height = (width * 0.05 + title.size().y + width * 0.035).max(rect.height() * 0.26);
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, vec2(width, height)),
                CornerRadius {
                    nw: round.nw,
                    ne: round.ne,
                    sw: 0,
                    se: 0,
                },
                cover.band,
            );
            ui.painter().galley(
                rect.min + vec2(width * 0.06, width * 0.05),
                title,
                text_colour,
            );
        }
        (MixStyle::Station, Some(photo)) => {
            ui.painter().rect_filled(rect, round, cover.band);
            let side = width * 0.66;
            let circle =
                Rect::from_min_size(rect.min + Vec2::splat(width * 0.17), Vec2::splat(side));
            let shadow = egui::epaint::Shadow {
                offset: [0, (width * 0.02) as i8],
                blur: (width * 0.06) as u8,
                spread: 0,
                color: Color32::from_black_alpha(71),
            };
            ui.painter()
                .add(shadow.as_shape(circle, CornerRadius::same((side / 2.0).min(127.0) as u8)));
            widgets::paint_cover(
                ui,
                &app.palette,
                Some(photo),
                circle,
                side / 2.0,
                theme::Icon::Music,
                Some(app.backend.art()),
            );
        }
        // No photo, or a mix of its own: the name large on the gradient.
        _ => {
            paint_gradient(ui, rect, radius, cover.from, cover.to);
            let title = widgets::ellipsized(
                ui,
                name,
                theme::bold(width * 0.15),
                Color32::WHITE,
                width * (1.0 - 2.0 * 0.08),
                3,
            );
            let top = rect.center().y - title.size().y / 2.0;
            ui.painter()
                .galley(pos2(rect.left() + width * 0.08, top), title, Color32::WHITE);
        }
    }
    true
}

/// `linear-gradient(135deg, from, to)` under `radial-gradient(120% 90% at 0 0,
/// from, transparent 70%)`, as a mesh of rounded rings so its corners stay
/// round and the radial part stays smooth.
fn paint_gradient(ui: &egui::Ui, rect: Rect, radius: f32, from: Color32, to: Color32) {
    const RINGS: u32 = 10;
    const CORNER_STEPS: u32 = 6;
    let colour_at = |point: egui::Pos2| {
        let uv = (point - rect.min) / rect.size();
        let along = ((uv.x + uv.y) / 2.0).clamp(0.0, 1.0);
        let base = from.lerp_to_gamma(to, along);
        let distance = ((uv.x / 1.2).powi(2) + (uv.y / 0.9).powi(2)).sqrt();
        base.lerp_to_gamma(from, (1.0 - distance / 0.7).clamp(0.0, 1.0))
    };
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut outline = Vec::new();
    for (corner, start) in [
        (rect.right_top() + vec2(-radius, radius), -90.0_f32),
        (rect.right_bottom() + vec2(-radius, -radius), 0.0),
        (rect.left_bottom() + vec2(radius, -radius), 90.0),
        (rect.left_top() + vec2(radius, radius), 180.0),
    ] {
        for step in 0..=CORNER_STEPS {
            let angle = (start + 90.0 * step as f32 / CORNER_STEPS as f32).to_radians();
            outline.push(corner + vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    let center = rect.center();
    let mut mesh = Mesh::default();
    let count = outline.len() as u32;
    for ring in 0..=RINGS {
        let t = ring as f32 / RINGS as f32;
        for point in &outline {
            let at = center + (*point - center) * t;
            mesh.colored_vertex(at, colour_at(at));
        }
    }
    for ring in 0..RINGS {
        for index in 0..count {
            let next = (index + 1) % count;
            let (a, b) = (ring * count + index, ring * count + next);
            let (c, d) = (a + count, b + count);
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(b, d, c);
        }
    }
    ui.painter().add(mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "4tZwfgrHOc3mvqYlEYSvVi";

    #[test]
    fn a_mixs_address_says_where_its_photo_is() {
        let pickasso = |path: &str| {
            format!("https://pickasso.spotifycdn.com/image/ab67c0de0000deef/dt/v1/img/{path}")
        };
        let hash = "ab6761610000e5eb1234567890abcdef12345678";
        assert_eq!(
            mix_source(&pickasso(&format!("daily/2/{hash}/en"))),
            Some(MixSource::Photo(format!("https://i.scdn.co/image/{hash}")))
        );
        assert_eq!(
            mix_source(&pickasso(&format!("radio/track/{ID}/en"))),
            Some(MixSource::Track(ID.into()))
        );
        assert_eq!(
            mix_source(&pickasso(&format!("radio/artist/{ID}/en?x=1"))),
            Some(MixSource::Artist(ID.into()))
        );
        assert_eq!(
            mix_source(&pickasso("mood/chill/en")),
            Some(MixSource::None)
        );
        assert_eq!(
            mix_source("https://daylist.spotifycdn.com/x"),
            Some(MixSource::None)
        );
        assert_eq!(mix_source("https://i.scdn.co/image/ab67"), None);
    }

    /// The band stays light enough for black text whatever the photo, and the
    /// gradient runs from lighter to darker.
    #[test]
    fn mix_colours_keep_the_band_light_and_the_gradient_falling() {
        for rgb in [[12, 10, 40], [250, 240, 20], [128, 128, 128], [0, 0, 0]] {
            let (band, from, to) = mix_colours(rgb);
            let light = |c: Color32| hsl_of([c.r(), c.g(), c.b()]).2;
            assert!(light(band) >= 0.59 && light(band) <= 0.79, "{rgb:?}");
            assert!(light(from) > light(to), "{rgb:?}");
        }
    }
}

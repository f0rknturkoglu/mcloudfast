//! Liquid Glass, as mcloud fits it to macOS's own (theme.js `toolbarLens`,
//! state `player`, after github.com/emribilemir/liquid-glass-research): the
//! page behind the surface stays sharp and is bent at the rim, so the
//! outermost points show the page about one radius further in and the bend
//! flattens out half a radius inside; bright detail is pulled toward its
//! darker surroundings by a wide halo; a tone curve by luminance lifts black
//! and holds white down while keeping each colour's hue; and a highlight
//! runs along the top and bottom edges. No tint, border or shadow goes over
//! it.
//!
//! egui paints a layer's shapes in order into the window's framebuffer, so
//! when the glass's paint callback runs, everything under it is already
//! there. The callback copies that region into a texture, makes the halo
//! from it with a separable Gaussian at a fraction of its resolution, and
//! draws the glass back inside the surface's capsule.
//!
//! [`crate::settings::GlassEffect::Eco`] draws no glass, and a GPU that
//! cannot compile the shaders falls back to the same solid surface.

use std::sync::{Arc, Mutex};

use eframe::egui_glow;
use eframe::glow::{self, HasContext as _};
use egui::{Id, Rect};

use crate::settings::GlassEffect;

/// The halo's blur, in points: the shader mixes a min with it in at 70%.
const HALO_BLUR: f32 = 13.5;
/// Output luminance (0–255) at each 1/32 of input luminance.
const TONE: [f32; 33] = [
    32.0, 36.0, 45.0, 56.0, 65.0, 71.0, 78.0, 86.0, 90.0, 96.0, 102.0, 109.0, 116.0, 120.0, 122.0,
    125.0, 130.0, 135.0, 141.0, 147.0, 152.0, 156.0, 156.0, 157.0, 161.0, 167.0, 170.0, 170.0,
    174.0, 178.0, 179.0, 181.0, 185.0,
];
/// Colour gain by luminance: macOS's glass boosts dark colours more.
const GAIN: [f32; 33] = [
    1.2962, 1.2833, 1.2705, 1.2576, 1.2447, 1.2319, 1.219, 1.2062, 1.1933, 1.1805, 1.1676, 1.1547,
    1.1419, 1.129, 1.1162, 1.1033, 1.0905, 1.0776, 1.0647, 1.0519, 1.039, 1.0262, 1.0133, 1.0005,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
];
/// The highlight along the top and bottom edges, every half point inward.
const HIGHLIGHT: [f32; 15] = [
    0.13, 0.085, 0.032, 0.019, 0.014, 0.011, 0.009, 0.007, 0.005, 0.004, 0.003, 0.002, 0.001,
    0.0005, 0.0,
];

/// How finely the halo is made.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Quality {
    /// Screen pixels per blurred texel.
    downscale: i32,
    /// Horizontal and vertical pass pairs.
    iterations: u32,
}

impl Quality {
    fn of(effect: GlassEffect) -> Option<Self> {
        match effect {
            GlassEffect::Full => Some(Self {
                downscale: 2,
                iterations: 4,
            }),
            GlassEffect::Balanced => Some(Self {
                downscale: 4,
                iterations: 3,
            }),
            GlassEffect::Eco => None,
        }
    }

    /// The halo's standard deviation in screen pixels.
    fn sigma_px(self, pixels_per_point: f32) -> f32 {
        HALO_BLUR * pixels_per_point
    }

    /// How far apart, in blurred texels, a pass's taps sit for the halo to
    /// reach its blur over all the passes. The nine-tap kernel's standard
    /// deviation is 1.64 steps.
    fn spread(self, pixels_per_point: f32) -> f32 {
        self.sigma_px(pixels_per_point)
            / (self.downscale as f32 * 1.64 * (self.iterations as f32).sqrt())
    }

    /// How far outside the surface, in screen pixels, the halo reads the
    /// page: three standard deviations. The bend only reads inside it.
    fn reach_px(self, pixels_per_point: f32) -> i32 {
        (3.0 * self.sigma_px(pixels_per_point)).ceil() as i32 + 2
    }
}

/// Paints glass in the capsule `rect` under whatever the caller draws next.
/// Returns `false` when it drew nothing: the effect is Eco or the GPU could
/// not run it, and the caller's surface should be solid.
pub fn paint(ui: &egui::Ui, rect: Rect, effect: GlassEffect) -> bool {
    let Some(quality) = Quality::of(effect) else {
        return false;
    };
    let renderer = ui.ctx().data_mut(|data| {
        data.get_temp_mut_or_insert_with(Id::new("mcloud-glass"), || {
            Arc::new(Mutex::new(Renderer::Pending))
        })
        .clone()
    });
    if renderer
        .lock()
        .is_ok_and(|renderer| matches!(*renderer, Renderer::Failed))
    {
        return false;
    }
    let callback = egui_glow::CallbackFn::new(move |info, painter| {
        let Ok(mut renderer) = renderer.lock() else {
            return;
        };
        if let Renderer::Pending = *renderer {
            *renderer = match Resources::new(painter.gl()) {
                Ok(resources) => Renderer::Ready(Box::new(resources)),
                Err(error) => {
                    log::warn!("mcloud glass is unavailable, drawing solid surfaces: {error}");
                    Renderer::Failed
                }
            };
        }
        if let Renderer::Ready(resources) = &mut *renderer {
            resources.draw(painter, &info, quality);
        }
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(callback),
    });
    true
}

enum Renderer {
    /// No frame has drawn glass yet.
    Pending,
    Ready(Box<Resources>),
    /// The shaders did not build; the surfaces stay solid.
    Failed,
}

struct Target {
    texture: glow::Texture,
    size: [i32; 2],
}

struct Uniforms {
    tex: Option<glow::UniformLocation>,
    halo: Option<glow::UniformLocation>,
    rect: Option<glow::UniformLocation>,
    capture: Option<glow::UniformLocation>,
    ppp: Option<glow::UniformLocation>,
    tone: Option<glow::UniformLocation>,
    gain: Option<glow::UniformLocation>,
    highlight: Option<glow::UniformLocation>,
}

struct Resources {
    blur: glow::Program,
    blur_tex: Option<glow::UniformLocation>,
    blur_dir: Option<glow::UniformLocation>,
    composite: glow::Program,
    uniforms: Uniforms,
    vertex_array: glow::VertexArray,
    _quad: glow::Buffer,
    framebuffer: glow::Framebuffer,
    /// The page under the surface, at full resolution.
    capture: Target,
    /// The halo's two halves, at the effect's fraction of it.
    ping: Target,
    pong: Target,
    /// GLES wants an unsized internal format, desktop GL a sized one.
    internal_format: i32,
}

/// The attribute slot of the quad's corners in both programs.
const POSITION: u32 = 0;

const VERTEX: &str = r"
#if NEW_SHADER_INTERFACE
    #define I in
    #define O out
#else
    #define I attribute
    #define O varying
#endif
I vec2 a_pos;
O vec2 v_uv;
void main() {
    v_uv = a_pos * 0.5 + 0.5;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
";

const FRAGMENT_HEADER: &str = r"
#ifdef GL_ES
    #if defined(GL_FRAGMENT_PRECISION_HIGH) && GL_FRAGMENT_PRECISION_HIGH == 1
        precision highp float;
    #else
        precision mediump float;
    #endif
#endif
#if NEW_SHADER_INTERFACE
    in vec2 v_uv;
    out vec4 f_color;
    #define gl_FragColor f_color
    #define texture2D texture
#else
    varying vec2 v_uv;
#endif
";

/// Nine taps through five bilinear samples (Rákos' weights), along `u_dir`.
const BLUR: &str = r"
uniform sampler2D u_tex;
uniform vec2 u_dir;
void main() {
    vec4 sum = texture2D(u_tex, v_uv) * 0.2270270270;
    sum += texture2D(u_tex, v_uv + u_dir * 1.3846153846) * 0.3162162162;
    sum += texture2D(u_tex, v_uv - u_dir * 1.3846153846) * 0.3162162162;
    sum += texture2D(u_tex, v_uv + u_dir * 3.2307692308) * 0.0702702703;
    sum += texture2D(u_tex, v_uv - u_dir * 3.2307692308) * 0.0702702703;
    gl_FragColor = sum;
}
";

/// The glass inside the capsule `u_rect`. Positions are window pixels from
/// the bottom left, as `gl_FragCoord`; `u_ppp` turns them into points.
const COMPOSITE: &str = r"
uniform sampler2D u_tex;
uniform sampler2D u_halo;
uniform vec4 u_rect;
uniform vec4 u_capture;
uniform float u_ppp;
uniform float u_tone[33];
uniform float u_gain[33];
uniform float u_highlight[15];

float tone_at(float luma) {
    float f = clamp(luma, 0.0, 1.0) * 32.0;
    float value = u_tone[32];
    for (int i = 0; i < 32; i++) {
        if (f < float(i + 1)) {
            value = mix(u_tone[i], u_tone[i + 1], f - float(i));
            break;
        }
    }
    return value / 255.0;
}

float gain_at(float luma) {
    float f = clamp(luma, 0.0, 1.0) * 32.0;
    float value = u_gain[32];
    for (int i = 0; i < 32; i++) {
        if (f < float(i + 1)) {
            value = mix(u_gain[i], u_gain[i + 1], f - float(i));
            break;
        }
    }
    return value;
}

float highlight_at(float points) {
    float f = points * 2.0;
    float value = 0.0;
    for (int i = 0; i < 14; i++) {
        if (f < float(i + 1)) {
            value = mix(u_highlight[i], u_highlight[i + 1], f - float(i));
            break;
        }
    }
    return value;
}

void main() {
    vec2 p = gl_FragCoord.xy - u_rect.xy;
    float radius = u_rect.w * 0.5;
    // The nearest point on the capsule's spine; the rim is a radius away.
    vec2 spine = vec2(clamp(p.x, radius, u_rect.z - radius), radius);
    vec2 inward = spine - p;
    float distance = length(inward);
    float edge = radius - distance;
    float alpha = clamp(edge + 0.5, 0.0, 1.0);
    if (alpha <= 0.0) {
        discard;
    }
    vec2 normal = distance > 0.0001 ? inward / distance : vec2(0.0);

    // The bend: a circular profile, the page one radius further in at the
    // rim, flat from half a radius inside.
    float reach = 1.01 * radius;
    float flat_from = 0.495 * radius;
    float t = min(1.0, max(edge, 0.0) / flat_from);
    float shift = reach * (1.0 - sqrt((2.0 - t) * t));
    vec2 at = gl_FragCoord.xy + normal * shift;
    vec2 uv = (at - u_capture.xy) / u_capture.zw;
    vec3 core = texture2D(u_tex, uv).rgb;
    vec3 halo = texture2D(u_halo, uv).rgb;

    // Bright detail darkens toward its surroundings; dark detail stays.
    vec3 colour = mix(core, min(core, halo), 0.7);

    // The tone by luminance, keeping the hue.
    float luma = dot(colour, vec3(0.2126, 0.7152, 0.0722));
    colour = tone_at(luma) + (colour - luma) * gain_at(luma);

    // The highlight along the top and bottom edges, none at the round ends'
    // middles, raising the luminance as on grey while keeping the hue.
    float lit = highlight_at(max(edge, 0.0) / u_ppp) * pow(abs(normal.y), 3.5);
    float lit_luma = max(dot(colour, vec3(0.2126, 0.7152, 0.0722)), 0.001);
    colour *= 1.0 + lit * min(16.0, (2.0 - lit_luma) / lit_luma);

    // The hairline: the outermost pixel takes the page's own colour.
    vec3 page = texture2D(u_halo, (gl_FragCoord.xy - u_capture.xy) / u_capture.zw).rgb;
    colour = mix(colour, page, 0.8 * clamp(1.0 - edge, 0.0, 1.0));

    gl_FragColor = vec4(clamp(colour, 0.0, 1.0) * alpha, alpha);
}
";

impl Resources {
    fn new(gl: &glow::Context) -> Result<Self, String> {
        let version = egui_glow::ShaderVersion::get(gl);
        let header = format!(
            "{}\n#define NEW_SHADER_INTERFACE {}\n",
            version.version_declaration(),
            i32::from(version.is_new_shader_interface())
        );
        let vertex = format!("{header}{VERTEX}");
        let blur = program(gl, &vertex, &format!("{header}{FRAGMENT_HEADER}{BLUR}"))?;
        let composite = program(
            gl,
            &vertex,
            &format!("{header}{FRAGMENT_HEADER}{COMPOSITE}"),
        )?;
        let internal_format = if version.is_embedded() {
            glow::RGBA as i32
        } else {
            glow::RGBA8 as i32
        };
        // SAFETY: the callback runs on the thread that owns the context,
        // with it current, and every object made here belongs to it.
        unsafe {
            let vertex_array = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vertex_array));
            let quad = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad));
            let corners: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
            let bytes: Vec<u8> = corners.iter().flat_map(|c| c.to_ne_bytes()).collect();
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            // Both programs bind `a_pos` here (see `program`).
            gl.enable_vertex_attrib_array(POSITION);
            gl.vertex_attrib_pointer_f32(POSITION, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let framebuffer = gl.create_framebuffer()?;
            let uniform = |name| gl.get_uniform_location(composite, name);
            Ok(Self {
                blur_tex: gl.get_uniform_location(blur, "u_tex"),
                blur_dir: gl.get_uniform_location(blur, "u_dir"),
                blur,
                uniforms: Uniforms {
                    tex: uniform("u_tex"),
                    halo: uniform("u_halo"),
                    rect: uniform("u_rect"),
                    capture: uniform("u_capture"),
                    ppp: uniform("u_ppp"),
                    tone: uniform("u_tone"),
                    gain: uniform("u_gain"),
                    highlight: uniform("u_highlight"),
                },
                composite,
                vertex_array,
                _quad: quad,
                framebuffer,
                capture: target(gl)?,
                ping: target(gl)?,
                pong: target(gl)?,
                internal_format,
            })
        }
    }

    fn draw(
        &mut self,
        painter: &egui_glow::Painter,
        info: &egui::PaintCallbackInfo,
        quality: Quality,
    ) {
        let gl = painter.gl();
        let [screen_w, screen_h] = info.screen_size_px.map(|side| side as i32);
        let glass = info.viewport_in_pixels();
        let clip = info.clip_rect_in_pixels();
        let ppp = info.pixels_per_point;
        let reach = quality.reach_px(ppp);
        let x0 = (glass.left_px - reach).max(0);
        let y0 = (glass.from_bottom_px - reach).max(0);
        let x1 = (glass.left_px + glass.width_px + reach).min(screen_w);
        let y1 = (glass.from_bottom_px + glass.height_px + reach).min(screen_h);
        let capture = [x1 - x0, y1 - y0];
        if capture[0] <= 0 || capture[1] <= 0 || clip.width_px <= 0 || clip.height_px <= 0 {
            return;
        }
        let blurred = capture.map(|side| (side + quality.downscale - 1) / quality.downscale);
        let spread = quality.spread(ppp);
        // SAFETY: as in `new`; egui restores its own state after the callback.
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.active_texture(glow::TEXTURE0);

            // The page under the glass, before anything is drawn over it.
            self.resize(gl, Which::Capture, capture);
            gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
            gl.bind_texture(glow::TEXTURE_2D, Some(self.capture.texture));
            gl.copy_tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, x0, y0, capture[0], capture[1]);

            // The halo.
            self.resize(gl, Which::Ping, blurred);
            self.resize(gl, Which::Pong, blurred);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.viewport(0, 0, blurred[0], blurred[1]);
            gl.use_program(Some(self.blur));
            gl.uniform_1_i32(self.blur_tex.as_ref(), 0);
            gl.bind_vertex_array(Some(self.vertex_array));
            // A tap's step in the source's texture coordinates: `spread`
            // blurred texels, which in the capture are `downscale` pixels.
            let step =
                |size: [i32; 2], texels: f32| [texels / size[0] as f32, texels / size[1] as f32];
            let mut source = (
                self.capture.texture,
                step(capture, spread * quality.downscale as f32),
            );
            for _ in 0..quality.iterations {
                for (into, horizontal) in [(Which::Ping, true), (Which::Pong, false)] {
                    let into = self.target(into).texture;
                    gl.framebuffer_texture_2d(
                        glow::FRAMEBUFFER,
                        glow::COLOR_ATTACHMENT0,
                        glow::TEXTURE_2D,
                        Some(into),
                        0,
                    );
                    gl.bind_texture(glow::TEXTURE_2D, Some(source.0));
                    let [dx, dy] = source.1;
                    if horizontal {
                        gl.uniform_2_f32(self.blur_dir.as_ref(), dx, 0.0);
                    } else {
                        gl.uniform_2_f32(self.blur_dir.as_ref(), 0.0, dy);
                    }
                    gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    source = (into, step(blurred, spread));
                }
            }

            // Back into the window, inside the glass and egui's clip.
            gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
            gl.viewport(
                glass.left_px,
                glass.from_bottom_px,
                glass.width_px,
                glass.height_px,
            );
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(
                clip.left_px,
                clip.from_bottom_px,
                clip.width_px,
                clip.height_px,
            );
            gl.enable(glow::BLEND);
            gl.use_program(Some(self.composite));
            let uniforms = &self.uniforms;
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.pong.texture));
            gl.uniform_1_i32(uniforms.halo.as_ref(), 1);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.capture.texture));
            gl.uniform_1_i32(uniforms.tex.as_ref(), 0);
            gl.uniform_4_f32(
                uniforms.rect.as_ref(),
                glass.left_px as f32,
                glass.from_bottom_px as f32,
                glass.width_px as f32,
                glass.height_px as f32,
            );
            gl.uniform_4_f32(
                uniforms.capture.as_ref(),
                x0 as f32,
                y0 as f32,
                capture[0] as f32,
                capture[1] as f32,
            );
            gl.uniform_1_f32(uniforms.ppp.as_ref(), ppp);
            gl.uniform_1_f32_slice(uniforms.tone.as_ref(), &TONE);
            gl.uniform_1_f32_slice(uniforms.gain.as_ref(), &GAIN);
            gl.uniform_1_f32_slice(uniforms.highlight.as_ref(), &HIGHLIGHT);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.bind_vertex_array(None);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    fn target(&self, which: Which) -> &Target {
        match which {
            Which::Capture => &self.capture,
            Which::Ping => &self.ping,
            Which::Pong => &self.pong,
        }
    }

    /// Gives a texture this size, keeping it when it already has it.
    fn resize(&mut self, gl: &glow::Context, which: Which, size: [i32; 2]) {
        let internal_format = self.internal_format;
        let target = match which {
            Which::Capture => &mut self.capture,
            Which::Ping => &mut self.ping,
            Which::Pong => &mut self.pong,
        };
        if target.size == size {
            return;
        }
        // SAFETY: as in `new`.
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(target.texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format,
                size[0],
                size[1],
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
        }
        target.size = size;
    }
}

#[derive(Clone, Copy)]
enum Which {
    Capture,
    Ping,
    Pong,
}

/// An empty, linearly filtered texture clamped at its edges, sized later.
fn target(gl: &glow::Context) -> Result<Target, String> {
    // SAFETY: as in `Resources::new`.
    unsafe {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(Target {
            texture,
            size: [0, 0],
        })
    }
}

fn program(gl: &glow::Context, vertex: &str, fragment: &str) -> Result<glow::Program, String> {
    // SAFETY: as in `Resources::new`.
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, vertex),
            (glow::FRAGMENT_SHADER, fragment),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.bind_attrib_location(program, POSITION, "a_pos");
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eco_draws_no_glass_and_both_others_make_the_same_halo() {
        assert_eq!(Quality::of(GlassEffect::Eco), None);
        let full = Quality::of(GlassEffect::Full).unwrap();
        let balanced = Quality::of(GlassEffect::Balanced).unwrap();
        assert!(full.downscale < balanced.downscale);
        // Spread, scale and passes add up to the same blur either way.
        for quality in [full, balanced] {
            let sigma = quality.spread(2.0)
                * quality.downscale as f32
                * 1.64
                * (quality.iterations as f32).sqrt();
            assert!((sigma - HALO_BLUR * 2.0).abs() < 1e-3);
            assert!(quality.reach_px(2.0) as f32 >= 3.0 * HALO_BLUR * 2.0);
        }
    }

    #[test]
    fn the_tone_lifts_black_and_holds_white_down() {
        assert_eq!(TONE.len(), 33);
        assert_eq!((TONE[0], TONE[32]), (32.0, 185.0));
        assert!(TONE.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(GAIN.iter().all(|gain| (1.0..1.3).contains(gain)));
    }
}

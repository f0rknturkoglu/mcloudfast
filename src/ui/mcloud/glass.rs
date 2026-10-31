//! Liquid glass: the page behind a surface, blurred, saturated and bent at
//! the surface's edges, drawn on the GPU before the surface's own tint.
//!
//! egui paints a layer's shapes in order into the window's framebuffer, so
//! when the glass's paint callback runs, everything under it is already
//! there. The callback copies that region into a texture, blurs it at a
//! fraction of its resolution with a separable Gaussian, ping-ponging
//! between two textures, and draws it back inside the surface's rounded
//! rectangle. Near the edge each pixel samples a little further inside,
//! as a lens would, which is what makes the glass read as liquid rather
//! than frosted. The tint, hairlines and highlights egui draws next.
//!
//! [`crate::settings::GlassEffect::Eco`] draws no glass at all, and a GPU
//! that cannot compile the shaders falls back to the same solid surface.

use std::sync::{Arc, Mutex};

use eframe::egui_glow;
use eframe::glow::{self, HasContext as _};
use egui::{Id, Rect};

use crate::settings::GlassEffect;

/// How finely an effect blurs, and how far its edges bend.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Quality {
    /// Screen pixels per blurred texel.
    downscale: i32,
    /// Horizontal and vertical pass pairs.
    iterations: u32,
    /// How far apart the kernel's taps sit, in blurred texels.
    spread: f32,
    /// How far, in points, the edge pulls its samples inward.
    refraction: f32,
}

impl Quality {
    fn of(effect: GlassEffect) -> Option<Self> {
        match effect {
            GlassEffect::Full => Some(Self {
                downscale: 2,
                iterations: 3,
                spread: 2.0,
                refraction: 10.0,
            }),
            GlassEffect::Balanced => Some(Self {
                downscale: 4,
                iterations: 2,
                spread: 1.6,
                refraction: 8.0,
            }),
            GlassEffect::Eco => None,
        }
    }

    /// How far outside the surface, in screen pixels, the blur and the bend
    /// read the page. The nine-tap kernel reaches four taps either side.
    fn reach_px(self, pixels_per_point: f32) -> i32 {
        let blur = self.iterations as f32 * 4.0 * self.spread * self.downscale as f32;
        (blur + self.refraction * pixels_per_point).ceil() as i32
    }
}

/// Paints glass in `rect`, rounded by `radius` points, under whatever the
/// caller draws next. Returns `false` when it drew nothing: the effect is
/// Eco or the GPU could not run it, and the caller's surface should be
/// solid.
pub fn paint(ui: &egui::Ui, rect: Rect, radius: f32, effect: GlassEffect) -> bool {
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
            resources.draw(painter, &info, radius, quality);
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

struct Resources {
    blur: glow::Program,
    blur_tex: Option<glow::UniformLocation>,
    blur_dir: Option<glow::UniformLocation>,
    composite: glow::Program,
    composite_tex: Option<glow::UniformLocation>,
    composite_rect: Option<glow::UniformLocation>,
    composite_capture: Option<glow::UniformLocation>,
    composite_radius: Option<glow::UniformLocation>,
    composite_refraction: Option<glow::UniformLocation>,
    composite_bend: Option<glow::UniformLocation>,
    vertex_array: glow::VertexArray,
    _quad: glow::Buffer,
    framebuffer: glow::Framebuffer,
    /// The page under the surface, at full resolution.
    capture: Target,
    /// The blur's two halves, at the effect's fraction of it.
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

/// The blurred page inside the rounded rectangle, bent inward near its
/// edge and a little more saturated and brighter, as Apple's glass is.
/// Positions are window pixels from the bottom left, as `gl_FragCoord`.
const COMPOSITE: &str = r"
uniform sampler2D u_tex;
uniform vec4 u_rect;
uniform vec4 u_capture;
uniform float u_radius;
uniform float u_refraction;
uniform float u_bend;

float rounded(vec2 p, vec2 half_size, float radius) {
    vec2 q = abs(p) - half_size + radius;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void main() {
    vec2 half_size = u_rect.zw * 0.5;
    vec2 p = gl_FragCoord.xy - (u_rect.xy + half_size);
    float d = rounded(p, half_size, u_radius);
    float alpha = clamp(0.5 - d, 0.0, 1.0);
    if (alpha <= 0.0) {
        discard;
    }
    vec2 normal = vec2(
        rounded(p + vec2(1.0, 0.0), half_size, u_radius)
            - rounded(p - vec2(1.0, 0.0), half_size, u_radius),
        rounded(p + vec2(0.0, 1.0), half_size, u_radius)
            - rounded(p - vec2(0.0, 1.0), half_size, u_radius));
    normal /= max(length(normal), 0.0001);
    float edge = clamp(1.0 + d / u_bend, 0.0, 1.0);
    vec2 at = gl_FragCoord.xy - normal * edge * edge * u_refraction;
    vec3 colour = texture2D(u_tex, (at - u_capture.xy) / u_capture.zw).rgb;
    float luma = dot(colour, vec3(0.2126, 0.7152, 0.0722));
    colour = clamp(mix(vec3(luma), colour, 1.6) * 1.06, 0.0, 1.0);
    gl_FragColor = vec4(colour * alpha, alpha);
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
            Ok(Self {
                blur_tex: gl.get_uniform_location(blur, "u_tex"),
                blur_dir: gl.get_uniform_location(blur, "u_dir"),
                blur,
                composite_tex: gl.get_uniform_location(composite, "u_tex"),
                composite_rect: gl.get_uniform_location(composite, "u_rect"),
                composite_capture: gl.get_uniform_location(composite, "u_capture"),
                composite_radius: gl.get_uniform_location(composite, "u_radius"),
                composite_refraction: gl.get_uniform_location(composite, "u_refraction"),
                composite_bend: gl.get_uniform_location(composite, "u_bend"),
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
        radius: f32,
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
                step(capture, quality.spread * quality.downscale as f32),
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
                    source = (into, step(blurred, quality.spread));
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
            gl.bind_texture(glow::TEXTURE_2D, Some(self.pong.texture));
            gl.uniform_1_i32(self.composite_tex.as_ref(), 0);
            gl.uniform_4_f32(
                self.composite_rect.as_ref(),
                glass.left_px as f32,
                glass.from_bottom_px as f32,
                glass.width_px as f32,
                glass.height_px as f32,
            );
            gl.uniform_4_f32(
                self.composite_capture.as_ref(),
                x0 as f32,
                y0 as f32,
                capture[0] as f32,
                capture[1] as f32,
            );
            gl.uniform_1_f32(self.composite_radius.as_ref(), radius * ppp);
            gl.uniform_1_f32(self.composite_refraction.as_ref(), quality.refraction * ppp);
            gl.uniform_1_f32(self.composite_bend.as_ref(), radius * ppp * 0.9);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.bind_vertex_array(None);
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
    fn eco_draws_no_glass_and_full_reaches_further_than_balanced() {
        assert_eq!(Quality::of(GlassEffect::Eco), None);
        let full = Quality::of(GlassEffect::Full).unwrap();
        let balanced = Quality::of(GlassEffect::Balanced).unwrap();
        assert!(full.downscale < balanced.downscale);
        assert!(full.reach_px(2.0) > 0 && balanced.reach_px(2.0) > 0);
        // The capture reaches past the blur and the bend, never less.
        assert!(balanced.reach_px(2.0) as f32 >= balanced.refraction * 2.0);
    }
}

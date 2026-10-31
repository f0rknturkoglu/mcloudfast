//! The Browse page: category cards for genres, moods and activities,
//! styled after Apple Music's "Göz At" / Browse view with vibrant
//! diagonal gradient tiles.

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

struct Category {
    name: &'static str,
    query: &'static str,
    c1: Color32,
    c2: Color32,
    icon: Icon,
}

const CATEGORIES: &[Category] = &[
    Category {
        name: "Pop",
        query: "Pop",
        c1: Color32::from_rgb(225, 48, 108),
        c2: Color32::from_rgb(131, 58, 180),
        icon: Icon::Music,
    },
    Category {
        name: "Hip-Hop",
        query: "Hip-Hop",
        c1: Color32::from_rgb(230, 81, 0),
        c2: Color32::from_rgb(255, 152, 0),
        icon: Icon::Music,
    },
    Category {
        name: "Rock",
        query: "Rock",
        c1: Color32::from_rgb(183, 28, 28),
        c2: Color32::from_rgb(136, 14, 79),
        icon: Icon::McloudAlbum,
    },
    Category {
        name: "Dance & Electronic",
        query: "Electronic",
        c1: Color32::from_rgb(0, 180, 216),
        c2: Color32::from_rgb(0, 119, 182),
        icon: Icon::Radio,
    },
    Category {
        name: "R&B",
        query: "R&B",
        c1: Color32::from_rgb(74, 20, 140),
        c2: Color32::from_rgb(123, 31, 162),
        icon: Icon::Heart,
    },
    Category {
        name: "Indie & Alternative",
        query: "Indie",
        c1: Color32::from_rgb(46, 125, 50),
        c2: Color32::from_rgb(27, 94, 32),
        icon: Icon::Music,
    },
    Category {
        name: "Chill",
        query: "Chill",
        c1: Color32::from_rgb(79, 195, 247),
        c2: Color32::from_rgb(2, 136, 209),
        icon: Icon::Headphones,
    },
    Category {
        name: "Mood",
        query: "Mood",
        c1: Color32::from_rgb(255, 112, 67),
        c2: Color32::from_rgb(244, 81, 30),
        icon: Icon::Sparkles,
    },
    Category {
        name: "Workout",
        query: "Workout",
        c1: Color32::from_rgb(255, 87, 34),
        c2: Color32::from_rgb(230, 74, 25),
        icon: Icon::Heart,
    },
    Category {
        name: "Focus",
        query: "Focus",
        c1: Color32::from_rgb(48, 63, 159),
        c2: Color32::from_rgb(26, 35, 126),
        icon: Icon::Clock,
    },
    Category {
        name: "Jazz",
        query: "Jazz",
        c1: Color32::from_rgb(141, 110, 99),
        c2: Color32::from_rgb(78, 52, 46),
        icon: Icon::Music,
    },
    Category {
        name: "Classical",
        query: "Classical",
        c1: Color32::from_rgb(55, 71, 79),
        c2: Color32::from_rgb(38, 50, 56),
        icon: Icon::Music,
    },
    Category {
        name: "Metal",
        query: "Metal",
        c1: Color32::from_rgb(33, 33, 33),
        c2: Color32::from_rgb(93, 0, 0),
        icon: Icon::McloudAlbum,
    },
    Category {
        name: "K-Pop",
        query: "K-Pop",
        c1: Color32::from_rgb(255, 64, 129),
        c2: Color32::from_rgb(197, 17, 98),
        icon: Icon::Heart,
    },
    Category {
        name: "Country",
        query: "Country",
        c1: Color32::from_rgb(161, 136, 127),
        c2: Color32::from_rgb(109, 76, 65),
        icon: Icon::Music,
    },
    Category {
        name: "Latin",
        query: "Latin",
        c1: Color32::from_rgb(255, 179, 0),
        c2: Color32::from_rgb(255, 61, 0),
        icon: Icon::Music,
    },
    Category {
        name: "Decades",
        query: "80s",
        c1: Color32::from_rgb(156, 39, 176),
        c2: Color32::from_rgb(103, 58, 183),
        icon: Icon::Clock,
    },
    Category {
        name: "Gaming",
        query: "Gaming",
        c1: Color32::from_rgb(124, 77, 255),
        c2: Color32::from_rgb(101, 31, 255),
        icon: Icon::Gamepad,
    },
    Category {
        name: "Sleep & Ambient",
        query: "Ambient",
        c1: Color32::from_rgb(26, 35, 126),
        c2: Color32::from_rgb(13, 71, 161),
        icon: Icon::Clock,
    },
    Category {
        name: "Blues",
        query: "Blues",
        c1: Color32::from_rgb(13, 71, 161),
        c2: Color32::from_rgb(0, 150, 136),
        icon: Icon::Music,
    },
];

fn rounded_diagonal_mesh(rect: Rect, r: f32, c1: Color32, c2: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let center = rect.center();
    let c_mid = c1.lerp_to_gamma(c2, 0.5);
    mesh.colored_vertex(center, c_mid);

    let corners = [
        (
            pos2(rect.right() - r, rect.bottom() - r),
            0.0,
            std::f32::consts::FRAC_PI_2,
        ),
        (
            pos2(rect.left() + r, rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ),
        (
            pos2(rect.left() + r, rect.top() + r),
            std::f32::consts::PI,
            3.0 * std::f32::consts::FRAC_PI_2,
        ),
        (
            pos2(rect.right() - r, rect.top() + r),
            3.0 * std::f32::consts::FRAC_PI_2,
            2.0 * std::f32::consts::PI,
        ),
    ];

    let mut perimeter_indices = Vec::new();
    for (cp, start_angle, end_angle) in corners {
        const SEGMENTS: usize = 4;
        for i in 0..=SEGMENTS {
            let angle = start_angle + (end_angle - start_angle) * (i as f32 / SEGMENTS as f32);
            let p = pos2(cp.x + r * angle.cos(), cp.y + r * angle.sin());
            let tx = (p.x - rect.left()) / rect.width();
            let ty = (p.y - rect.top()) / rect.height();
            let t = ((tx + ty) * 0.5).clamp(0.0, 1.0);
            let col = c1.lerp_to_gamma(c2, t);
            let idx = mesh.vertices.len() as u32;
            mesh.colored_vertex(p, col);
            perimeter_indices.push(idx);
        }
    }

    let n = perimeter_indices.len();
    for i in 0..n {
        let next = perimeter_indices[(i + 1) % n];
        let curr = perimeter_indices[i];
        mesh.add_triangle(0, curr, next);
    }

    mesh
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(8.0);

    theme::section_title(ui, &palette, &gettext(locale, "Browse"));
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(gettext(locale, "Explore by genre and mood"))
            .size(13.5)
            .color(palette.secondary),
    );
    ui.add_space(18.0);

    let total_width = ui.available_width();
    let min_card_width = 160.0;
    let gap = 16.0;
    let columns = (((total_width + gap) / (min_card_width + gap)).floor() as usize).max(2);
    let card_width =
        ((total_width - gap * (columns - 1) as f32) / columns as f32).max(min_card_width);
    let card_height = (card_width * 0.52).clamp(94.0, 116.0);

    for chunk in CATEGORIES.chunks(columns) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for category in chunk {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(card_width, card_height), Sense::click());
                let hovered = response.hovered();

                let (c1, c2) = if hovered {
                    (
                        category.c1.linear_multiply(1.15),
                        category.c2.linear_multiply(1.15),
                    )
                } else {
                    (category.c1, category.c2)
                };

                let mesh = rounded_diagonal_mesh(rect, 8.0, c1, c2);
                ui.painter().add(egui::Shape::mesh(mesh));

                let icon_box = Rect::from_center_size(
                    pos2(rect.right() - 26.0, rect.bottom() - 22.0),
                    vec2(36.0, 36.0),
                );
                theme::paint_icon(
                    ui,
                    category.icon,
                    icon_box,
                    36.0,
                    Color32::from_white_alpha(35),
                );

                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(8),
                    Stroke::new(
                        1.0,
                        Color32::from_white_alpha(if hovered { 40 } else { 15 }),
                    ),
                    StrokeKind::Inside,
                );

                ui.painter().line_segment(
                    [
                        pos2(rect.left() + 8.0, rect.top() + 1.0),
                        pos2(rect.right() - 8.0, rect.top() + 1.0),
                    ],
                    Stroke::new(1.0, Color32::from_white_alpha(35)),
                );

                let font_id = egui::FontId::new(17.0, egui::FontFamily::Proportional);
                ui.painter().text(
                    pos2(rect.left() + 14.0, rect.top() + 14.0),
                    egui::Align2::LEFT_TOP,
                    category.name,
                    font_id,
                    Color32::WHITE,
                );

                if response.clicked() {
                    app.actions.push(Action::Search(category.query.to_string()));
                    app.actions.push(Action::Open(Page::Search));
                }
            }
        });
        ui.add_space(gap);
    }
}

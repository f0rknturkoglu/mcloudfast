//! The Browse page: category tiles for genres, moods and activities, laid
//! out as Spotify's own "Browse all", which mcloud opens from the search
//! field.

use egui::{Color32, CornerRadius, Rect, Sense, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

struct Category {
    name: &'static str,
    query: &'static str,
    colour: Color32,
    icon: Icon,
}

const CATEGORIES: &[Category] = &[
    Category {
        name: "Pop",
        query: "Pop",
        colour: Color32::from_rgb(225, 48, 108),
        icon: Icon::Music,
    },
    Category {
        name: "Hip-Hop",
        query: "Hip-Hop",
        colour: Color32::from_rgb(230, 81, 0),
        icon: Icon::Music,
    },
    Category {
        name: "Rock",
        query: "Rock",
        colour: Color32::from_rgb(183, 28, 28),
        icon: Icon::McloudAlbum,
    },
    Category {
        name: "Dance & Electronic",
        query: "Electronic",
        colour: Color32::from_rgb(0, 180, 216),
        icon: Icon::Radio,
    },
    Category {
        name: "R&B",
        query: "R&B",
        colour: Color32::from_rgb(74, 20, 140),
        icon: Icon::Heart,
    },
    Category {
        name: "Indie & Alternative",
        query: "Indie",
        colour: Color32::from_rgb(46, 125, 50),
        icon: Icon::Music,
    },
    Category {
        name: "Chill",
        query: "Chill",
        colour: Color32::from_rgb(79, 195, 247),
        icon: Icon::Headphones,
    },
    Category {
        name: "Mood",
        query: "Mood",
        colour: Color32::from_rgb(255, 112, 67),
        icon: Icon::Sparkles,
    },
    Category {
        name: "Workout",
        query: "Workout",
        colour: Color32::from_rgb(255, 87, 34),
        icon: Icon::Heart,
    },
    Category {
        name: "Focus",
        query: "Focus",
        colour: Color32::from_rgb(48, 63, 159),
        icon: Icon::Clock,
    },
    Category {
        name: "Jazz",
        query: "Jazz",
        colour: Color32::from_rgb(141, 110, 99),
        icon: Icon::Music,
    },
    Category {
        name: "Classical",
        query: "Classical",
        colour: Color32::from_rgb(55, 71, 79),
        icon: Icon::Music,
    },
    Category {
        name: "Metal",
        query: "Metal",
        colour: Color32::from_rgb(33, 33, 33),
        icon: Icon::McloudAlbum,
    },
    Category {
        name: "K-Pop",
        query: "K-Pop",
        colour: Color32::from_rgb(255, 64, 129),
        icon: Icon::Heart,
    },
    Category {
        name: "Country",
        query: "Country",
        colour: Color32::from_rgb(161, 136, 127),
        icon: Icon::Music,
    },
    Category {
        name: "Latin",
        query: "Latin",
        colour: Color32::from_rgb(255, 179, 0),
        icon: Icon::Music,
    },
    Category {
        name: "Decades",
        query: "80s",
        colour: Color32::from_rgb(156, 39, 176),
        icon: Icon::Clock,
    },
    Category {
        name: "Gaming",
        query: "Gaming",
        colour: Color32::from_rgb(124, 77, 255),
        icon: Icon::Gamepad,
    },
    Category {
        name: "Sleep & Ambient",
        query: "Ambient",
        colour: Color32::from_rgb(26, 35, 126),
        icon: Icon::Clock,
    },
    Category {
        name: "Blues",
        query: "Blues",
        colour: Color32::from_rgb(13, 71, 161),
        icon: Icon::Music,
    },
];

/// Spotify's "Browse all" grid, which mcloud leaves as it is: at most
/// 1104pt wide, centred, tiles about 258 by 144pt and 24pt apart, each a
/// flat colour with its name at the top left.
const MAX_WIDTH: f32 = 1104.0;
const MIN_TILE: f32 = 240.0;
const GAP: f32 = 24.0;
const ASPECT: f32 = 0.56;

/// How many tiles fit across `width`, and how wide each is.
fn columns(width: f32) -> (usize, f32) {
    let count = (((width + GAP) / (MIN_TILE + GAP)).floor() as usize).max(2);
    let tile = (width - GAP * (count as f32 - 1.0)) / count as f32;
    (count, tile)
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(56.0);

    let width = ui.available_width().min(MAX_WIDTH);
    let inset = (ui.available_width() - width) / 2.0;
    let (count, tile_width) = columns(width);
    let tile_height = tile_width * ASPECT;

    ui.horizontal(|ui| {
        ui.add_space(inset);
        theme::text(
            ui,
            gettext(locale, "Browse all").as_ref(),
            theme::bold(24.0),
            palette.text,
        );
    });
    ui.add_space(12.0);

    for chunk in CATEGORIES.chunks(count) {
        ui.horizontal(|ui| {
            ui.add_space(inset - ui.spacing().item_spacing.x);
            ui.spacing_mut().item_spacing.x = GAP;
            for category in chunk {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(tile_width, tile_height), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, category.name)
                });
                let fill = if response.hovered() {
                    category.colour.lerp_to_gamma(Color32::WHITE, 0.08)
                } else {
                    category.colour
                };
                ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
                // Where Spotify tilts a cover into the corner, the
                // category's glyph.
                let glyph = Rect::from_center_size(
                    pos2(
                        rect.right() - tile_height * 0.32,
                        rect.bottom() - tile_height * 0.3,
                    ),
                    vec2(tile_height * 0.5, tile_height * 0.5),
                );
                theme::paint_icon(
                    ui,
                    category.icon,
                    glyph,
                    tile_height * 0.5,
                    Color32::from_white_alpha(46),
                );
                let name = crate::ui::widgets::ellipsized(
                    ui,
                    category.name,
                    theme::bold(24.0),
                    Color32::WHITE,
                    tile_width - 32.0 - tile_height * 0.4,
                    2,
                );
                ui.painter()
                    .galley(rect.min + vec2(16.0, 16.0), name, Color32::WHITE);
                theme::focus_ring(ui, &response);
                if response.clicked() {
                    app.actions.push(Action::Search(category.query.to_string()));
                    app.actions.push(Action::Open(Page::Search));
                }
            }
        });
        ui.add_space(GAP - ui.spacing().item_spacing.y);
    }
}

#[cfg(test)]
mod tests {
    /// Four tiles across Spotify's 1104pt column, fewer in a narrow window.
    #[test]
    fn browse_fits_spotifys_four_columns() {
        let (count, tile) = super::columns(1104.0);
        assert_eq!(count, 4);
        assert!((tile - 258.0).abs() < 0.01);
        assert_eq!(super::columns(700.0).0, 2);
    }
}

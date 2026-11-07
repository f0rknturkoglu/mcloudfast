//! The Browse page: category tiles for genres, moods and activities, laid
//! out as Spotify's own "Browse all", which mcloud opens from the search
//! field.

use egui::{Color32, CornerRadius, Rect, Sense, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable, Page};
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
/// mcloud's tiles, from Apple Music's Browse Categories (user.css): at least
/// 180pt, 16:9 and 16pt apart, across the whole page.
const MCLOUD_MIN_TILE: f32 = 180.0;
const MCLOUD_GAP: f32 = 16.0;
const MCLOUD_ASPECT: f32 = 9.0 / 16.0;

/// How many tiles fit across `width`, and how wide each is.
fn columns(width: f32) -> (usize, f32) {
    columns_of(width, MIN_TILE, GAP)
}

fn columns_of(width: f32, min_tile: f32, gap: f32) -> (usize, f32) {
    let count = (((width + gap) / (min_tile + gap)).floor() as usize).max(2);
    let tile = (width - gap * (count as f32 - 1.0)) / count as f32;
    (count, tile)
}

/// What a tile under mcloud shows: Spotify's own category, or this page's
/// flat one until Spotify has answered, or when it can't.
struct McloudTile {
    name: String,
    query: String,
    colour: Color32,
    picture: Option<(String, [u8; 3])>,
}

fn mcloud_tiles(app: &mut App, ctx: &egui::Context) -> Vec<McloudTile> {
    app.request_browse_tiles();
    if matches!(app.browse_tiles, Loadable::Failed(_)) {
        // Nothing else draws this page again to ask once more.
        ctx.request_repaint_after(crate::app::BROWSE_TILES_RETRY);
    }
    if let Loadable::Loaded(tiles) = &app.browse_tiles {
        return tiles
            .iter()
            .map(|tile| McloudTile {
                name: tile.name.clone(),
                query: tile.name.clone(),
                colour: Color32::from_rgb(tile.colour[0], tile.colour[1], tile.colour[2]),
                picture: tile.image_url.clone().map(|url| (url, tile.colour)),
            })
            .collect();
    }
    CATEGORIES
        .iter()
        .map(|category| McloudTile {
            name: category.name.to_string(),
            query: category.query.to_string(),
            colour: category.colour,
            picture: None,
        })
        .collect()
}

/// Apple Music's Browse Categories under mcloud: Spotify's picture as a
/// duotone in the tile's colour, the name small at the bottom left, a faint
/// rim, a small heading.
fn mcloud_grid(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(14.0);
    theme::text(
        ui,
        gettext(locale, "Browse all").as_ref(),
        theme::bold(15.0),
        palette.text,
    );
    ui.add_space(10.0);
    let tiles = mcloud_tiles(app, ui.ctx());
    let width = ui.available_width();
    let (count, tile_width) = columns_of(width, MCLOUD_MIN_TILE, MCLOUD_GAP);
    let tile_height = tile_width * MCLOUD_ASPECT;
    let rows = tiles.len().div_ceil(count);
    let (block, _) = ui.allocate_exact_size(
        vec2(width, (tile_height + MCLOUD_GAP) * rows as f32 - MCLOUD_GAP),
        Sense::hover(),
    );
    let rim = if palette.dark {
        Color32::from_white_alpha(82)
    } else {
        Color32::from_rgba_unmultiplied(112, 112, 112, 54)
    };
    for (index, tile) in tiles.iter().enumerate() {
        let rect = Rect::from_min_size(
            block.min
                + vec2(
                    (tile_width + MCLOUD_GAP) * (index % count) as f32,
                    (tile_height + MCLOUD_GAP) * (index / count) as f32,
                ),
            vec2(tile_width, tile_height),
        );
        let response = ui.interact(rect, ui.id().with(("browse-tile", index)), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tile.name.as_str())
        });
        if ui.is_rect_visible(rect) {
            let fill = if response.hovered() {
                tile.colour.lerp_to_gamma(Color32::WHITE, 0.08)
            } else {
                tile.colour
            };
            ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
            let art = app.backend.art();
            let picture = tile
                .picture
                .as_ref()
                .and_then(|(url, colour)| app.duotone_tiles.texture(ui.ctx(), art, url, *colour));
            if let Some(texture) = picture {
                egui::Image::from_texture(&texture)
                    .corner_radius(CornerRadius::same(8))
                    .tint(if response.hovered() {
                        Color32::from_gray(255)
                    } else {
                        Color32::from_gray(245)
                    })
                    .paint_at(ui, rect);
            }
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                egui::Stroke::new(0.5, rim),
                egui::StrokeKind::Inside,
            );
            let name = crate::ui::widgets::ellipsized(
                ui,
                &tile.name,
                theme::bold(13.0),
                Color32::WHITE,
                tile_width - 24.0,
                2,
            );
            let at = pos2(rect.left() + 12.0, rect.bottom() - 12.0 - name.size().y);
            ui.painter().galley(at, name, Color32::WHITE);
        }
        theme::focus_ring(ui, &response);
        if response.clicked() {
            app.actions.push(Action::Search(tile.query.clone()));
            app.actions.push(Action::Open(Page::Search));
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    if super::mcloud::active(ui.ctx()) {
        mcloud_grid(app, ui);
        return;
    }
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
            // Spacing is added between items, not before the first one. Less
            // than nothing here would also move the page's left edge with it,
            // and each row would start further left than the one before.
            ui.add_space(inset);
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
        // mcloud's, at least 180pt across the page.
        let (count, tile) = super::columns_of(1108.0, 180.0, 16.0);
        assert_eq!(count, 5);
        assert!((tile - 208.8).abs() < 0.01);
    }
}

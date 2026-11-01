//! The artist page as mcloud draws it (user.css `#am-artist`), after Apple
//! Music: the artist's header picture across the page, fading into the
//! artist's colour, which fills the whole window; the name centred over its
//! bottom with a white Play and a round Follow star; then the latest
//! release beside a three-row grid of top songs, and shelves of albums,
//! singles, appearances and similar artists.
//!
//! Spotify's header picture comes with the artist's play counts
//! ([`crate::session_reads::PageExtras`]); without one, the portrait stands
//! in, round and centred, as mcloud's avatar hero does.

use std::sync::Arc;

use egui::{Color32, CornerRadius, Id, Layout, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{Album, Artist, PlayableItem, pick_image};
use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, ArtistPage, DiscographyFilter, Loadable, Page, RowContext};
use crate::theme::{self, Icon};
use crate::ui::widgets::{self, PAGE_PADDING};

/// The page's side inset, as mcloud's shelves keep it.
const INSET: f32 = 32.0;
const SHELF_GAP: f32 = 30.0;
const CARD: f32 = 160.0;
const ARTIST_CARD: f32 = 150.0;
const CARD_GAP: f32 = 18.0;
const SONG_COLUMN: f32 = 310.0;
const SONG_ROWS: usize = 3;
const LATEST_ART: f32 = 150.0;
const WHITE_60: Color32 = Color32::from_rgba_premultiplied(153, 153, 153, 153);
const NAME: f32 = 40.0;
const NAME_COLOR: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe4);
const PLAY: f32 = 72.0;
const PLAY_ICON: f32 = 35.0;
const SIDE_BUTTON: f32 = 48.0;

/// The colours the artist page takes from its picture, as mcloud's
/// `setTint` does: the picture's hue at 38% lightness across the top 40% of
/// the window, deepening toward 14% lightness 100% further down, so the
/// window's bottom is 60% of the way there. A grey picture keeps a grey
/// page; until the colour is read, a neutral grey stands in.
pub(in crate::ui) fn page_colours(app: &mut App, id: &str) -> (Color32, Color32) {
    let artist = app
        .artist_pages
        .get(id)
        .and_then(|page| page.artist.get())
        .or_else(|| app.known_artist(id));
    let url = artist.and_then(|artist| {
        app.artist_headers
            .get(&artist.uri)
            .cloned()
            .or_else(|| pick_image(&artist.images, 64).map(str::to_string))
    });
    match app.artwork_colours(url.as_deref()) {
        Some(colours) => {
            let (top, bottom) = tint(colours.page);
            (top, top.lerp_to_gamma(bottom, 0.6))
        }
        None => (
            Color32::from_rgb(0x3a, 0x3a, 0x3a),
            Color32::from_rgb(0x3a, 0x3a, 0x3a).lerp_to_gamma(app.palette.window, 0.6),
        ),
    }
}

/// mcloud's top and deep colours for a picture's colour.
fn tint(rgb: [u8; 3]) -> (Color32, Color32) {
    let [r, g, b] = rgb.map(|channel| f32::from(channel) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let lightness = (max + min) / 2.0;
    let (hue, saturation) = if max == min {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let saturation = if lightness > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let hue = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (hue * 60.0, saturation)
    };
    // Raising a grey's saturation would turn hue 0 into red.
    let saturation = if saturation < 0.1 {
        saturation
    } else {
        saturation.clamp(0.35, 0.85)
    };
    // CSS rounds both to whole numbers.
    let hue = hue.round();
    let saturation = (saturation * 100.0).round() / 100.0;
    (
        hsl(hue, saturation, 0.38),
        hsl(hue, (saturation * 80.0).round() / 100.0, 0.14),
    )
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color32 {
    let a = saturation * lightness.min(1.0 - lightness);
    let channel = |n: f32| {
        let k = (n + hue / 30.0) % 12.0;
        let value = lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (value * 255.0).round() as u8
    };
    Color32::from_rgb(channel(0.0), channel(8.0), channel(4.0))
}

pub(in crate::ui) fn loaded(
    app: &mut App,
    ui: &mut egui::Ui,
    page: &ArtistPage,
    artist: &Artist,
    id: &str,
) {
    // The header picture arrives with the play counts.
    app.request_play_counts(&artist.uri);
    hero(app, ui, artist);
    about_sheet(app, ui.ctx(), artist);
    ui.add_space(10.0);

    let latest = latest_release(page);
    // The latest release and the top songs share the first row.
    ui.horizontal_top(|ui| {
        ui.add_space(INSET - PAGE_PADDING);
        if let Some(album) = latest {
            ui.allocate_ui_with_layout(
                vec2(300.0, 0.0),
                Layout::top_down(egui::Align::Min),
                |ui| latest_card(app, ui, album),
            );
            ui.add_space(SHELF_GAP);
        }
        ui.vertical(|ui| top_songs(app, ui, page, artist));
    });
    ui.add_space(SHELF_GAP);

    let all = page
        .albums
        .get(DiscographyFilter::All.groups())
        .or_else(|| page.albums.get(page.filter.groups()));
    if let Some(list) = all {
        let mut seen = std::collections::HashSet::new();
        let releases: Vec<&Album> = list
            .items
            .iter()
            .filter(|album| seen.insert(album.name.to_lowercase()))
            .collect();
        let (singles, albums): (Vec<&Album>, Vec<&Album>) = releases
            .iter()
            .partition(|album| album.album_type.as_deref() == Some("single"));
        album_shelf(
            app,
            ui,
            "artist-albums",
            &gettext(app.locale, "Albums"),
            &albums,
        );
        album_shelf(
            app,
            ui,
            "artist-singles",
            &gettext(app.locale, "Singles & EPs"),
            &singles,
        );
        if list.can_load_more() && !list.loading {
            ui.horizontal(|ui| {
                ui.add_space(INSET - PAGE_PADDING);
                if theme::soft_button(
                    ui,
                    &app.palette,
                    None,
                    &gettext(app.locale, "Load more"),
                    false,
                )
                .clicked()
                {
                    app.actions
                        .push(Action::LoadMoreArtistAlbums(id.to_string()));
                }
            });
            ui.add_space(SHELF_GAP);
        }
    }
    if let Some(list) = page.albums.get(DiscographyFilter::AppearsOn.groups()) {
        let appears: Vec<&Album> = list.items.iter().collect();
        album_shelf(
            app,
            ui,
            "artist-appears",
            &gettext(app.locale, "Appears On"),
            &appears,
        );
    }
    if let Loadable::Loaded(related) = &page.related
        && !related.is_empty()
    {
        let title = gettext(app.locale, "Fans also like");
        shelf(ui, "artist-related", &title, |ui| {
            for artist in related {
                let image = pick_image(&artist.images, app.art_px(ARTIST_CARD));
                let response = card(app, ui, image, &artist.name, None, ARTIST_CARD, true);
                if response.clicked() {
                    app.actions
                        .push(Action::Open(Page::Artist(artist.id.clone())));
                }
                egui::Popup::context_menu(&response)
                    .id(ui.make_persistent_id(("related-artist-menu", &artist.uri)))
                    .frame(widgets::menu_frame(&app.palette))
                    .show(|ui| {
                        widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None)
                    });
            }
        });
    }
}

/// The picture across the page, fading out into the artist's colour, the
/// name over its bottom, Play and Follow under the name, and the floating
/// back and "…" buttons at the top.
fn hero(app: &mut App, ui: &mut egui::Ui, artist: &Artist) {
    let palette = app.palette;
    let locale = app.locale;
    let page = ui.max_rect();
    // The page's own width, its side padding included.
    let full = Rect::from_min_max(
        pos2(page.left() - PAGE_PADDING, page.top()),
        pos2(page.right() + PAGE_PADDING, page.bottom()),
    );
    let width = full.width();
    let header = app.artist_headers.get(&artist.uri).cloned();
    let height = if header.is_some() {
        (width * 0.41 + 68.0).max(380.0)
    } else {
        (ui.ctx().content_rect().height() * 0.62).clamp(380.0, 620.0)
    };
    let (reserved, _) = ui.allocate_exact_size(vec2(page.width(), height), Sense::hover());
    let rect = Rect::from_min_max(
        pos2(full.left(), reserved.top() - 4.0),
        pos2(full.right(), reserved.bottom()),
    );

    if let Some(url) = header.as_deref() {
        header_picture(app, ui, url, rect);
    } else if let Some(url) = pick_image(&artist.images, app.art_px(250.0)) {
        // The portrait, round and centred.
        let side = (width * 0.17).clamp(170.0, 250.0);
        let portrait = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + 48.0 + side / 2.0),
            Vec2::splat(side),
        );
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 12],
                blur: 40,
                spread: 0,
                color: Color32::from_black_alpha(102),
            }
            .as_shape(portrait, CornerRadius::same(127)),
        );
        widgets::paint_cover(
            ui,
            &palette,
            Some(url),
            portrait,
            side / 2.0,
            Icon::User,
            Some(app.backend.art()),
        );
    }

    // The name, centred over the hero's bottom, and the buttons under it,
    // measured on Apple Music's header: the name a fixed 40pt, ⓘ 48, Play
    // 72 and Follow 48 apart by 24 on one centre line, Play under the name.
    let name = widgets::ellipsized(
        ui,
        &artist.name,
        theme::bold(NAME),
        NAME_COLOR,
        width - 2.0 * INSET,
        2,
    );
    let actions_y = rect.bottom() - 26.0 - PLAY / 2.0;
    let name_bottom = actions_y - PLAY / 2.0 - 15.5;
    let name_pos = pos2(
        rect.center().x - name.size().x / 2.0,
        name_bottom - name.size().y,
    );
    ui.painter().galley(
        name_pos + vec2(0.0, 1.0),
        name.clone(),
        Color32::from_black_alpha(50),
    );
    ui.painter().galley(name_pos, name, NAME_COLOR);

    // Play is a white disc with the triangle in the artist's colour.
    let artist_id = artist.id.clone();
    let (top_tint, _) = page_colours(app, &artist_id);
    let play = Rect::from_center_size(pos2(rect.center().x, actions_y), Vec2::splat(PLAY));
    let context_here = app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let response = ui.interact(play, Id::new("mcloud-artist-play"), Sense::click());
    ui.painter().add(
        egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(51),
        }
        .as_shape(play, CornerRadius::same((PLAY / 2.0) as u8)),
    );
    let scale = if response.is_pointer_button_down_on() {
        0.94
    } else {
        1.0
    };
    ui.painter()
        .circle_filled(play.center(), PLAY / 2.0 * scale, Color32::WHITE);
    let (icon, label) = if playing_here {
        (Icon::McloudPause, gettext(locale, "Pause"))
    } else {
        (Icon::McloudHeroPlay, gettext(locale, "Play"))
    };
    // The triangle's box sits 1/18 of its width right of the disc's centre.
    theme::paint_icon(
        ui,
        icon,
        play.translate(vec2(PLAY_ICON / 18.0, 0.0)),
        PLAY_ICON * scale,
        top_tint,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref()));
    if response.on_hover_text(label.as_ref()).clicked() {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: artist.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }

    // ⓘ opens the About sheet; it waits for Spotify's overview.
    let about = app.artist_about.contains_key(&artist.uri);
    let info = Rect::from_center_size(
        pos2(play.left() - 24.0 - SIDE_BUTTON / 2.0, actions_y),
        Vec2::splat(SIDE_BUTTON),
    );
    let label = gettext(locale, "About");
    let response = side_button(
        ui,
        info,
        "mcloud-artist-info",
        Icon::McloudInfo,
        20.0,
        about,
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, about, label.as_ref()));
    if about && response.on_hover_text(label.as_ref()).clicked() {
        open_about(ui.ctx(), &artist.uri);
    }

    let following = app.is_saved(&artist.uri).unwrap_or(false);
    let follow = Rect::from_center_size(
        pos2(play.right() + 24.0 + SIDE_BUTTON / 2.0, actions_y),
        Vec2::splat(SIDE_BUTTON),
    );
    let label = if following {
        pgettext(locale, "artist", "Following")
    } else {
        pgettext(locale, "artist", "Follow")
    };
    let response = side_button(
        ui,
        follow,
        "mcloud-artist-follow",
        Icon::McloudStar,
        20.5,
        true,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, following, label.as_ref())
    });
    if response.on_hover_text(label.as_ref()).clicked() {
        app.actions.push(Action::ToggleSaved(artist.uri.clone()));
    }

    floating_buttons(app, ui, full, artist, top_tint);
}

/// ⓘ and Follow: a faint glass disc with a hairline inside its edge.
fn side_button(
    ui: &mut egui::Ui,
    rect: Rect,
    id: &str,
    icon: Icon,
    size: f32,
    enabled: bool,
) -> egui::Response {
    let response = ui.interact(rect, Id::new(id), Sense::click());
    let painter = ui.painter();
    let fill = if enabled && response.hovered() {
        36
    } else {
        20
    };
    painter.circle_filled(
        rect.center(),
        rect.width() / 2.0,
        Color32::from_white_alpha(fill),
    );
    painter.circle_stroke(
        rect.center(),
        rect.width() / 2.0 - 0.5,
        egui::Stroke::new(1.0, Color32::from_white_alpha(13)),
    );
    let color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(110)
    };
    theme::paint_icon(ui, icon, rect, size, color);
    theme::focus_ring(ui, &response);
    response
}

const ABOUT_ID: &str = "mcloud-artist-about";
/// The About card's widest, as Apple Music's (W below).
const ABOUT_WIDTH: f32 = 704.0;
const ABOUT_RADIUS: f32 = 16.0;

fn open_about(ctx: &egui::Context, uri: &str) {
    ctx.data_mut(|data| data.insert_temp(Id::new(ABOUT_ID), uri.to_string()));
}

fn close_about(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<String>(Id::new(ABOUT_ID)));
}

/// The ⓘ sheet, after Apple Music's About sheet as mcloud measures it: a
/// card at most 704pt wide, 52pt below the window's top and 40pt above its
/// bottom, over a dim. Its square photo fades from 0.495W into the card's
/// colour, the photo's own, held dark enough for white text. Scrolling, the
/// photo drifts up at 0.4x and fades out between 0.257W and 0.541W.
fn about_sheet(app: &mut App, ctx: &egui::Context, artist: &Artist) {
    let open = ctx.data(|data| data.get_temp::<String>(Id::new(ABOUT_ID)));
    if open.as_deref() != Some(artist.uri.as_str()) {
        return;
    }
    let Some(about) = app.artist_about.get(&artist.uri).cloned() else {
        close_about(ctx);
        return;
    };
    let locale = app.locale;
    let screen = ctx.content_rect();
    let width = (screen.width() - 32.0).min(ABOUT_WIDTH);
    let card = Rect::from_min_max(
        pos2(screen.center().x - width / 2.0, screen.top() + 52.0),
        pos2(
            screen.center().x + width / 2.0,
            (screen.bottom() - 40.0).max(screen.top() + 252.0),
        ),
    );
    let background = about
        .photo
        .as_deref()
        .and_then(|url| app.artwork_colours(Some(url)))
        .map(|colours| {
            let [r, g, b] = colours.common;
            sheet_colour(Color32::from_rgb(r, g, b))
        })
        .unwrap_or(Color32::from_rgb(0x3a, 0x3a, 0x3a));
    // macOS draws the labels, chips and biography as vibrant text, added
    // onto the card's colour.
    let lighter = |amount: u8| {
        Color32::from_rgb(
            background.r().saturating_add(amount),
            background.g().saturating_add(amount),
            background.b().saturating_add(amount),
        )
    };
    let mut close = ctx.input(|input| input.key_pressed(egui::Key::Escape));

    egui::Area::new(Id::new("mcloud-artist-about-sheet"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            let dim = ui.interact(screen, Id::new("mcloud-artist-about-dim"), Sense::click());
            if dim.clicked()
                && dim
                    .interact_pointer_pos()
                    .is_some_and(|pos| !card.contains(pos))
            {
                close = true;
            }
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(125));
            ui.painter().add(
                egui::epaint::Shadow {
                    offset: [0, 22],
                    blur: 70,
                    spread: 0,
                    color: Color32::from_black_alpha(140),
                }
                .as_shape(card, CornerRadius::same(ABOUT_RADIUS as u8)),
            );
            ui.painter()
                .rect_filled(card, CornerRadius::same(ABOUT_RADIUS as u8), background);
            // The card swallows clicks, so only the dim closes it.
            ui.interact(card, Id::new("mcloud-artist-about-card"), Sense::click());

            let photo_slot = ui.painter().add(egui::Shape::Noop);
            let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card));
            card_ui.set_clip_rect(card);
            let scroll = egui::ScrollArea::vertical()
                .id_salt(("mcloud-artist-about-scroll", &artist.uri))
                .auto_shrink([false, false])
                .show(&mut card_ui, |ui| {
                    let top = if about.photo.is_some() {
                        width * 0.6906
                    } else {
                        64.0
                    };
                    ui.add_space(top);
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 20,
                            right: 20,
                            top: 0,
                            bottom: 40,
                        })
                        .show(ui, |ui| {
                            ui.set_width(width - 40.0);
                            about_body(ui, locale, artist, &about, background, &lighter);
                        });
                });
            if let Some(url) = about.photo.as_deref()
                && let Some(texture) = picture(app, ui, url, Vec2::splat(width))
            {
                let mesh = about_photo(texture, card, scroll.state.offset.y);
                ui.painter().set(photo_slot, egui::Shape::mesh(mesh));
            }

            let close_rect = Rect::from_min_size(card.min + vec2(15.5, 16.0), Vec2::splat(30.0));
            let label = gettext(locale, "Close");
            let response = ui.interact(
                close_rect,
                Id::new("mcloud-artist-about-close"),
                Sense::click(),
            );
            let fill = if response.hovered() { 150 } else { 121 };
            ui.painter().circle_filled(
                close_rect.center(),
                15.0,
                Color32::from_rgba_unmultiplied(120, 120, 120, fill),
            );
            theme::paint_icon(ui, Icon::McloudClose, close_rect, 20.0, Color32::WHITE);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
            });
            theme::focus_ring(ui, &response);
            if response.on_hover_text(label.as_ref()).clicked() {
                close = true;
            }
        });
    if close {
        close_about(ctx);
    }
}

/// The sheet's text: the name, the counts and cities, and the biography.
fn about_body(
    ui: &mut egui::Ui,
    locale: crate::i18n::Locale,
    artist: &Artist,
    about: &crate::session_reads::ArtistAbout,
    background: Color32,
    lighter: &dyn Fn(u8) -> Color32,
) {
    let width = ui.available_width();
    ui.spacing_mut().item_spacing.y = 0.0;
    let name = widgets::ellipsized(ui, &artist.name, theme::bold(26.0), NAME_COLOR, width, 3);
    let (rect, _) = ui.allocate_exact_size(vec2(width, name.size().y), Sense::hover());
    ui.painter().galley(rect.min, name, NAME_COLOR);

    let label_color = lighter(0x48);
    let fact = |ui: &mut egui::Ui, label: &str, value: &str| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            theme::text(ui, label, theme::semibold(11.0), label_color);
            theme::text(ui, value, theme::semibold(14.0), NAME_COLOR);
        });
    };
    let counts: Vec<(String, String)> = [
        (
            gettext(locale, "Monthly listeners"),
            about.monthly_listeners,
        ),
        (gettext(locale, "Followers"), about.followers),
    ]
    .into_iter()
    .filter_map(|(label, count)| Some((label.to_string(), crate::util::format_count(count?))))
    .collect();
    if !counts.is_empty() {
        ui.add_space(15.0);
        let column = (width - 20.0) / 2.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            for (label, value) in &counts {
                ui.allocate_ui_with_layout(
                    vec2(column, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| fact(ui, label, value),
                );
            }
        });
    }
    if !about.top_cities.is_empty() {
        ui.add_space(if counts.is_empty() { 15.0 } else { 11.5 });
        theme::text(
            ui,
            gettext(locale, "Top cities").as_ref(),
            theme::semibold(11.0),
            label_color,
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for city in &about.top_cities {
                let galley =
                    ui.painter()
                        .layout_no_wrap(city.clone(), theme::semibold(12.0), NAME_COLOR);
                let size = vec2(galley.size().x + 24.0, 25.0);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().rect(
                    rect,
                    CornerRadius::same(13),
                    lighter(0x14),
                    egui::Stroke::new(1.0, lighter(0x1a)),
                    egui::StrokeKind::Inside,
                );
                ui.painter()
                    .galley(rect.center() - galley.size() / 2.0, galley, NAME_COLOR);
            }
        });
    }
    if !about.biography.is_empty() {
        ui.add_space(36.0);
        theme::text(
            ui,
            gettext(locale, "About").as_ref(),
            theme::bold(16.0),
            NAME_COLOR,
        );
        ui.add_space(12.0);
        let mut job = egui::text::LayoutJob::single_section(
            about.biography.clone(),
            egui::TextFormat {
                font_id: theme::regular(13.0),
                color: lighter(0xb0),
                line_height: Some(18.0),
                ..Default::default()
            },
        );
        job.wrap.max_width = width;
        let galley = ui.painter().layout_job(job);
        let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
        ui.painter().galley(rect.min, galley, background);
    }
}

/// The card's colour from the photo's: no brighter than 42%, so white
/// text always reads on it.
fn sheet_colour(colour: Color32) -> Color32 {
    let max = colour.r().max(colour.g()).max(colour.b());
    if max <= 107 {
        return colour;
    }
    let scale = 107.0 / f32::from(max);
    let channel = |value: u8| (f32::from(value) * scale).round() as u8;
    Color32::from_rgb(
        channel(colour.r()),
        channel(colour.g()),
        channel(colour.b()),
    )
}

/// The sheet's photo as a mesh inside the card's rounded corners: square
/// and W wide, reflected below itself, solid to 0.495W and fading out by
/// 1.295W; it drifts up at 0.4x the scroll and fades out between 0.257W and
/// 0.541W of it.
fn about_photo(texture: egui::load::SizedTexture, card: Rect, scrolled: f32) -> egui::Mesh {
    let width = card.width();
    let mut mesh = egui::Mesh::with_texture(texture.id);
    let opacity = 1.0 - ((scrolled - 0.257 * width) / (0.284 * width)).clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return mesh;
    }
    let uv = crop(texture.size, Vec2::splat(width), 0.5);
    let top = card.top() - 0.4 * scrolled.min(1000.0);
    let end = top + 1.295 * width;
    let first = card.top();
    let last = card.bottom().min(end);
    if last <= first {
        return mesh;
    }
    // Where the fade, the reflection and the corners bend the strip.
    let mut rows: Vec<f32> = vec![first, last, top + 0.495 * width, top + width];
    for step in 0..=8 {
        let t = step as f32 / 8.0 * ABOUT_RADIUS;
        rows.push(card.top() + t);
        rows.push(card.bottom() - t);
    }
    rows.retain(|y| (first..=last).contains(y));
    rows.sort_by(f32::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let inset = |y: f32| {
        let from_edge = (y - card.top()).min(card.bottom() - y);
        if from_edge >= ABOUT_RADIUS {
            return 0.0;
        }
        let dy = ABOUT_RADIUS - from_edge.max(0.0);
        ABOUT_RADIUS - (ABOUT_RADIUS * ABOUT_RADIUS - dy * dy).max(0.0).sqrt()
    };
    for (index, y) in rows.iter().enumerate() {
        let offset = y - top;
        // Below the photo its reflection: the row as far above its bottom.
        let photo_y = if offset > width {
            2.0 * width - offset
        } else {
            offset
        };
        let v = uv.top() + photo_y / width * uv.height();
        let fade = 1.0 - ((offset - 0.495 * width) / (0.8 * width)).clamp(0.0, 1.0);
        let colour = Color32::WHITE.gamma_multiply(fade * opacity);
        let x = inset(*y);
        for (side, u) in [
            (card.left() + x, uv.left() + x / width * uv.width()),
            (card.right() - x, uv.right() - x / width * uv.width()),
        ] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pos2(side, *y),
                uv: pos2(u, v),
                color: colour,
            });
        }
        if index > 0 {
            let i = index as u32 * 2;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    mesh
}

/// The header picture: 53% of the width in height, fading out from 38% of
/// its height down into the colour behind it.
fn header_picture(app: &App, ui: &egui::Ui, url: &str, rect: Rect) {
    let width = rect.width();
    let media = Rect::from_min_size(rect.min, vec2(width, width * 0.53));
    let Some(texture) = picture(app, ui, url, media.size()) else {
        return;
    };
    let uv = crop(texture.size, media.size(), 0.22);
    let painter = ui.painter().with_clip_rect(media.intersect(ui.clip_rect()));
    let solid = media.top() + media.height() * 0.379;
    let v_at = |y: f32| uv.top() + (y - media.top()) / media.height() * uv.height();
    painter.image(
        texture.id,
        Rect::from_min_max(media.min, pos2(media.right(), solid)),
        Rect::from_min_max(uv.min, pos2(uv.right(), v_at(solid))),
        Color32::WHITE,
    );
    // The fade: a strip of quads whose alpha falls smoothly to nothing.
    let mut mesh = egui::Mesh::with_texture(texture.id);
    let steps = 16;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let y = solid + (media.bottom() - solid) * t;
        let alpha = 1.0 - t;
        let color = Color32::WHITE.gamma_multiply(alpha * alpha * (3.0 - 2.0 * alpha));
        for x in [media.left(), media.right()] {
            let u = if x == media.left() {
                uv.left()
            } else {
                uv.right()
            };
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pos2(x, y),
                uv: pos2(u, v_at(y)),
                color,
            });
        }
        if step > 0 {
            let i = (step as u32) * 2;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Back on the left, and Share and "…" sharing one capsule on the right,
/// floating over the hero in a glass of the artist's colour, where mcloud
/// hides the toolbar.
fn floating_buttons(app: &mut App, ui: &mut egui::Ui, full: Rect, artist: &Artist, tint: Color32) {
    let locale = app.locale;
    let fill = tint
        .lerp_to_gamma(Color32::WHITE, 0.12)
        .gamma_multiply(0.85);
    let top = full.top() + 12.0;
    let back = Rect::from_min_size(pos2(full.left() + 12.0, top), Vec2::splat(36.0));
    let actions = Rect::from_min_size(pos2(full.right() - 14.0 - 72.0, top), vec2(72.0, 36.0));
    ui.painter().circle_filled(back.center(), 18.0, fill);
    ui.painter()
        .rect_filled(actions, CornerRadius::same(18), fill);
    let share = Rect::from_min_size(actions.min, Vec2::splat(36.0));
    let more = Rect::from_min_size(pos2(actions.center().x, actions.top()), Vec2::splat(36.0));
    for (rect, icon, label) in [
        (back, Icon::McloudLeft, gettext(locale, "Back")),
        (share, Icon::McloudShare, gettext(locale, "Copy link")),
        (more, Icon::McloudMore, gettext(locale, "More")),
    ] {
        let response = ui.interact(
            rect,
            Id::new(("mcloud-artist-float", label.as_ref())),
            Sense::click(),
        );
        if response.hovered() {
            ui.painter()
                .circle_filled(rect.center(), 16.0, Color32::from_white_alpha(31));
        }
        theme::paint_icon(ui, icon, rect, 18.0, Color32::WHITE);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.as_ref())
        });
        theme::focus_ring(ui, &response);
        let response = response.on_hover_text(label.as_ref());
        match icon {
            Icon::McloudLeft => {
                if response.clicked() && app.can_go_back() {
                    app.actions.push(Action::Back);
                }
            }
            Icon::McloudShare => {
                if response.clicked() {
                    app.actions.push(Action::CopyLink(artist.uri.clone()));
                }
            }
            _ => {
                egui::Popup::menu(&response)
                    .frame(widgets::menu_frame(&app.palette))
                    .align(egui::RectAlign::BOTTOM_END)
                    .show(|ui| {
                        widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None)
                    });
            }
        }
    }
}

/// The picture at `url`, once loaded.
fn picture(app: &App, ui: &egui::Ui, url: &str, size: Vec2) -> Option<egui::load::SizedTexture> {
    app.backend.art().touch(url);
    let image = egui::Image::new(url)
        .texture_options(crate::images::COVER_TEXTURE)
        .show_loading_spinner(false);
    match image.load_for_size(ui.ctx(), size) {
        Ok(egui::load::TexturePoll::Ready { texture }) => {
            crate::images::forget_decoded(ui.ctx(), url);
            Some(texture)
        }
        _ => None,
    }
}

/// The part of a picture `image` in size that fills `target` without
/// stretching, as texture coordinates, its vertical centre at `focus`.
fn crop(image: Vec2, target: Vec2, focus: f32) -> Rect {
    let image_aspect = image.x / image.y.max(1.0);
    let target_aspect = target.x / target.y.max(1.0);
    if image_aspect > target_aspect {
        let width = target_aspect / image_aspect;
        Rect::from_min_max(pos2(0.5 - width / 2.0, 0.0), pos2(0.5 + width / 2.0, 1.0))
    } else {
        let height = image_aspect / target_aspect;
        let top = (focus - height / 2.0).clamp(0.0, 1.0 - height);
        Rect::from_min_max(pos2(0.0, top), pos2(1.0, top + height))
    }
}

/// The newest release in the discography loaded so far.
fn latest_release(page: &ArtistPage) -> Option<&Album> {
    page.albums
        .get(DiscographyFilter::All.groups())
        .or_else(|| page.albums.get(page.filter.groups()))?
        .items
        .iter()
        .filter(|album| album.release_date.is_some())
        .max_by(|a, b| a.release_date.cmp(&b.release_date))
}

fn title(ui: &mut egui::Ui, title: &str) {
    theme::text(ui, title, theme::bold(20.0), Color32::WHITE);
    ui.add_space(10.0);
}

/// A titled row of cards that scrolls sideways.
fn shelf(ui: &mut egui::Ui, id: &str, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(INSET - PAGE_PADDING);
        title(ui, label);
    });
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.add_space(INSET - PAGE_PADDING);
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                add_contents(ui);
                ui.add_space(INSET - PAGE_PADDING);
            });
        });
    ui.add_space(SHELF_GAP);
}

fn album_shelf(app: &mut App, ui: &mut egui::Ui, id: &str, label: &str, albums: &[&Album]) {
    if albums.is_empty() {
        return;
    }
    shelf(ui, id, label, |ui| {
        for album in albums {
            let image = pick_image(&album.images, app.art_px(CARD));
            let year = album.year().unwrap_or("").to_string();
            let response = card(app, ui, image, &album.name, Some(&year), CARD, false);
            if response.clicked() {
                app.actions
                    .push(Action::Open(Page::Album(album.id.clone())));
            }
            egui::Popup::context_menu(&response)
                .id(ui.make_persistent_id(("discography-menu", &album.uri)))
                .frame(widgets::menu_frame(&app.palette))
                .show(|ui| widgets::context_menu_items(ui, app, &album.uri, &album.name, None));
        }
    });
}

/// mcloud's card: the artwork with a faint inner ring, the name in two
/// lines at most, and a quieter line under it.
pub(crate) fn card(
    app: &mut App,
    ui: &mut egui::Ui,
    image: Option<&str>,
    name: &str,
    subtitle: Option<&str>,
    side: f32,
    round: bool,
) -> egui::Response {
    let name_galley = widgets::ellipsized(ui, name, theme::medium(13.0), Color32::WHITE, side, 2);
    let subtitle_galley = subtitle
        .filter(|text| !text.is_empty())
        .map(|text| widgets::ellipsized(ui, text, theme::regular(13.0), WHITE_60, side, 1));
    let height = side
        + 6.0
        + name_galley.size().y
        + subtitle_galley
            .as_ref()
            .map_or(0.0, |galley| galley.size().y + 2.0);
    let (rect, response) = ui.allocate_exact_size(vec2(side, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let art = Rect::from_min_size(rect.min, Vec2::splat(side));
        let radius = if round { side / 2.0 } else { 8.0 };
        let corner = CornerRadius::same(radius.min(127.0) as u8);
        widgets::paint_cover(
            ui,
            &app.palette,
            image,
            art,
            radius,
            if round { Icon::User } else { Icon::Music },
            Some(app.backend.art()),
        );
        ui.painter().rect_stroke(
            art,
            corner,
            egui::Stroke::new(1.0, Color32::from_white_alpha(18)),
            egui::StrokeKind::Inside,
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(art, corner, Color32::from_white_alpha(14));
        }
        let x = |galley: &Arc<egui::Galley>| {
            if round {
                rect.center().x - galley.size().x / 2.0
            } else {
                rect.left()
            }
        };
        let y = art.bottom() + 6.0;
        let name_height = name_galley.size().y;
        ui.painter().galley(
            pos2(x(&name_galley), y),
            name_galley.clone(),
            Color32::WHITE,
        );
        if let Some(galley) = subtitle_galley {
            ui.painter()
                .galley(pos2(x(&galley), y + name_height + 2.0), galley, WHITE_60);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The latest release: its artwork beside the date, the name, the number
/// of songs and a "+" that saves it.
fn latest_card(app: &mut App, ui: &mut egui::Ui, album: &Album) {
    title(ui, &gettext(app.locale, "Latest release"));
    let (row, response) = ui.allocate_exact_size(vec2(300.0, LATEST_ART), Sense::click());
    let art = Rect::from_min_size(row.min, Vec2::splat(LATEST_ART));
    widgets::paint_cover(
        ui,
        &app.palette,
        pick_image(&album.images, app.art_px(LATEST_ART)),
        art,
        8.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let left = art.right() + 16.0;
    let width = row.right() - left;
    let date = album
        .release_date
        .as_deref()
        .map(release_date)
        .unwrap_or_default();
    let date_color = Color32::from_white_alpha(166);
    let date = widgets::ellipsized(ui, &date, theme::semibold(11.0), date_color, width, 1);
    let name = widgets::ellipsized(
        ui,
        &album.name,
        theme::regular(15.0),
        Color32::WHITE,
        width,
        2,
    );
    let count = album
        .total_tracks
        .map(|total| app.locale.song_count(total))
        .unwrap_or_default();
    let count = widgets::ellipsized(ui, &count, theme::regular(15.0), WHITE_60, width, 1);
    let height = date.size().y + name.size().y + count.size().y + 4.0 + 10.0 + 28.0;
    let mut y = art.center().y - height / 2.0;
    for (galley, color) in [
        (date, date_color),
        (name, Color32::WHITE),
        (count, WHITE_60),
    ] {
        let size = galley.size();
        ui.painter().galley(pos2(left, y), galley, color);
        y += size.y + 2.0;
    }
    let add = Rect::from_min_size(pos2(left, y + 8.0), Vec2::splat(28.0));
    let saved = app.is_saved(&album.uri).unwrap_or(false);
    let add_response = ui.interact(
        add,
        Id::new(("mcloud-latest-add", &album.uri)),
        Sense::click(),
    );
    ui.painter()
        .circle_filled(add.center(), 14.0, Color32::from_white_alpha(38));
    theme::paint_icon(
        ui,
        if saved { Icon::Check } else { Icon::McloudPlus },
        add,
        14.0,
        Color32::WHITE,
    );
    if add_response.clicked() {
        app.actions.push(Action::ToggleSaved(album.uri.clone()));
    } else if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.actions
            .push(Action::Open(Page::Album(album.id.clone())));
    }
}

/// "2026-10-23" as "Oct 23, 2026"; a bare year or month stays as given.
fn release_date(date: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts[..] else {
        return date.to_string();
    };
    let Some(month) = month
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)))
    else {
        return date.to_string();
    };
    format!("{month} {}, {year}", day.trim_start_matches('0'))
}

/// The top songs in three rows that scroll sideways, a hairline under all
/// but each column's last.
fn top_songs(app: &mut App, ui: &mut egui::Ui, page: &ArtistPage, artist: &Artist) {
    let locale = app.locale;
    title(ui, &gettext(locale, "Top Songs"));
    let Loadable::Loaded(tracks) = &page.top_tracks else {
        widgets::skeleton_rows(ui, &app.palette, locale, SONG_ROWS);
        return;
    };
    let uris: Arc<[String]> = tracks
        .iter()
        .map(|track| track.uri.clone())
        .collect::<Vec<_>>()
        .into();
    let playing = app.current_track_uri();
    egui::ScrollArea::horizontal()
        .id_salt(("artist-top-songs", &artist.id))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 30.0;
                for (column, chunk) in tracks.chunks(SONG_ROWS).enumerate() {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (row, track) in chunk.iter().enumerate() {
                            let here = playing.as_deref() == Some(track.uri.as_str());
                            if song_row(app, ui, track, here, row + 1 < chunk.len()) {
                                app.actions.push(Action::PlayFromRow {
                                    context: RowContext::Uris(Arc::clone(&uris)),
                                    uri: track.uri.clone(),
                                    index: (column * SONG_ROWS + row) as u32,
                                });
                            }
                        }
                    });
                }
            });
        });
}

/// One of the top songs: artwork, name and artists, and "…". Returns
/// whether the row was clicked to play.
fn song_row(
    app: &mut App,
    ui: &mut egui::Ui,
    track: &crate::api::models::Track,
    here: bool,
    divider: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(SONG_COLUMN, 54.0), Sense::click());
    if divider {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, Color32::from_white_alpha(28)),
        );
    }
    let art = Rect::from_min_size(pos2(rect.left(), rect.center().y - 22.0), Vec2::splat(44.0));
    widgets::paint_cover(
        ui,
        &app.palette,
        track
            .album
            .as_ref()
            .and_then(|album| pick_image(&album.images, app.art_px(44.0))),
        art,
        5.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let color = if here {
        app.palette.accent
    } else {
        Color32::WHITE
    };
    let text_left = art.right() + 12.0;
    let width = rect.right() - 34.0 - text_left;
    let name = widgets::ellipsized(ui, &track.name, theme::regular(13.0), color, width, 1);
    let artists = track
        .artists
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let artists = widgets::ellipsized(ui, &artists, theme::regular(13.0), WHITE_60, width, 1);
    ui.painter().galley(
        pos2(text_left, rect.center().y - name.size().y),
        name,
        color,
    );
    ui.painter()
        .galley(pos2(text_left, rect.center().y), artists, WHITE_60);
    let more = Rect::from_center_size(
        pos2(rect.right() - 14.0, rect.center().y),
        Vec2::splat(28.0),
    );
    let more_response = ui.interact(
        more,
        Id::new(("mcloud-song-more", &track.uri)),
        Sense::click(),
    );
    theme::paint_icon(
        ui,
        Icon::McloudMore,
        more,
        18.0,
        Color32::from_white_alpha(192),
    );
    let item = PlayableItem::Track(track.clone());
    egui::Popup::menu(&more_response)
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| widgets::item_menu(ui, app, &item, None, None));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &track.name));
    response.clicked() && !more_response.hovered()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sheet's colour keeps a dark photo's and darkens a bright one to
    /// 42% at most, keeping its hue.
    #[test]
    fn the_about_sheet_stays_dark_enough_for_white_text() {
        let dark = Color32::from_rgb(40, 60, 80);
        assert_eq!(super::sheet_colour(dark), dark);
        let bright = super::sheet_colour(Color32::from_rgb(214, 107, 0));
        assert_eq!(bright, Color32::from_rgb(107, 54, 0));
    }

    /// The photo fills the card's width and is gone once scrolled 0.541W.
    #[test]
    fn the_about_photo_fades_out_as_the_sheet_scrolls() {
        let texture = egui::load::SizedTexture::new(egui::TextureId::default(), vec2(640.0, 640.0));
        let card = Rect::from_min_size(pos2(0.0, 52.0), vec2(704.0, 800.0));
        let mesh = super::about_photo(texture, card, 0.0);
        let left = mesh
            .vertices
            .iter()
            .map(|v| v.pos.x)
            .fold(f32::MAX, f32::min);
        let right = mesh
            .vertices
            .iter()
            .map(|v| v.pos.x)
            .fold(f32::MIN, f32::max);
        assert_eq!((left, right), (0.0, 704.0));
        // The top corners are rounded.
        assert!(mesh.vertices[0].pos.x > 0.0);
        assert!(super::about_photo(texture, card, 0.541 * 704.0 + 1.0).is_empty());
    }

    /// mcloud's tint: the hue at 38% and 14% lightness, its saturation held
    /// between 35% and 85%, and a grey kept grey.
    #[test]
    fn the_page_takes_mcloud_tint_of_the_picture() {
        let (top, deep) = super::tint([200, 30, 30]);
        assert_eq!(top, Color32::from_rgb(169, 25, 25));
        assert_eq!(deep, Color32::from_rgb(57, 15, 15));
        let (top, _) = super::tint([120, 121, 118]);
        assert_eq!(top, Color32::from_rgb(97, 98, 96));
    }

    #[test]
    fn the_banner_crops_around_its_focus() {
        let uv = crop(egui::vec2(640.0, 640.0), egui::vec2(1200.0, 400.0), 0.35);
        assert_eq!(uv.width(), 1.0);
        assert!((uv.height() - 1.0 / 3.0).abs() < 1e-4);
        assert!((uv.center().y - 0.35).abs() < 1e-4);
        // A picture wider than the band keeps its height and its centre.
        let uv = crop(egui::vec2(2000.0, 400.0), egui::vec2(1000.0, 400.0), 0.22);
        assert_eq!(uv.height(), 1.0);
        assert!((uv.center().x - 0.5).abs() < 1e-4);
    }

    #[test]
    fn release_dates_read_as_apple_music_writes_them() {
        assert_eq!(release_date("2026-10-23"), "Oct 23, 2026");
        assert_eq!(release_date("1984-11-09"), "Nov 9, 1984");
        assert_eq!(release_date("1984"), "1984");
    }
}

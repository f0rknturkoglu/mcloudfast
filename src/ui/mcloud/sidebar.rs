//! The sidebar as mcloud draws it (user.css `#am-sidebar`), after Apple
//! Music on macOS 26: a dark material over the page that lets about a
//! quarter of it through, navigation and the library's shelves led by
//! accent-coloured icons, the playlists with their artwork under a heading
//! with a "+", and the account with friends in the bottom-left corner.

use egui::{
    Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, UiBuilder, Vec2, pos2, vec2,
};

use crate::api::models::{Playlist, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

const ROW_HEIGHT: f32 = 32.0;
const ROW_RADIUS: u8 = 8;
const ROW_PADDING: f32 = 16.0;
const ICON_SIZE: f32 = 18.0;
const ART_SIZE: f32 = 22.0;
/// The gap between a row's icon or artwork and its name.
const ICON_GAP: f32 = 10.0;
/// The first row's top, level with the page under the toolbar.
const TOP: f32 = 76.0;
const SIDE_PADDING: f32 = 10.0;
const ACCOUNT_HEIGHT: f32 = 54.0;
/// The recently played playlists listed under the pinned ones while the
/// list is short, as mcloud's default.
const RECENT_PLAYLISTS: usize = 5;

const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
const HEADING: Color32 = Color32::from_rgba_premultiplied(92, 92, 92, 92);
const QUIET: Color32 = Color32::from_rgba_premultiplied(140, 140, 140, 140);

/// Checkpoints (in seconds) after `playing_context_uri` changes at which the
/// sidebar checks whether the newly playing playlist has moved to the top of
/// recents.
pub(crate) const RECENTS_POLL_DELAYS: [f64; 5] = [0.1, 0.4, 1.5, 5.0, 15.0];

#[derive(Clone, Default)]
struct RecentsSyncState {
    last_playing: Option<String>,
    changed_at: Option<f64>,
    displayed_recents: Vec<String>,
}

#[derive(Clone, Copy)]
struct CollapseScrollAnim {
    start_y: f32,
    target_y: f32,
    started_at: f64,
    duration: f32,
}

/// Ease-out cubic (`1 - (1 - t)^3`) used when scrolling back up before
/// collapsing a deep playlist list.
pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// What leads a row.
enum Leading<'a> {
    Icon(Icon),
    Art(Option<&'a str>, &'a crate::images::ArtLoader),
    /// "Show All": a small chevron in the icon column.
    Chevron,
    /// A folder: its glyph on the artwork's grey square.
    Folder,
}

/// What ends a row.
#[derive(Clone, Copy, PartialEq)]
enum RowEnd {
    Nothing,
    /// The playlist playing: a speaker.
    Playing,
    /// A grey pin.
    Pinned,
    /// A folder's disclosure, pointing down when it is open.
    Folder {
        open: bool,
    },
}

/// A row of the playlist list.
#[derive(Debug, PartialEq)]
enum SideRow {
    Playlist {
        index: usize,
        depth: u8,
    },
    Folder {
        id: String,
        name: String,
        open: bool,
        depth: u8,
    },
}

/// The opened list: what the short list shows stays where it was, then the
/// account's playlists in its own order with their folders, which can only be
/// read, not edited. Playlists the rootlist has not met yet wait at the end.
fn expanded_rows(
    playlists: &[Playlist],
    rootlist: &[crate::player::RootlistEntry],
    collapsed: &[String],
    first: &[usize],
) -> Vec<SideRow> {
    use crate::player::RootlistEntry;
    let mut rows: Vec<SideRow> = first
        .iter()
        .map(|&index| SideRow::Playlist { index, depth: 0 })
        .collect();
    let mut listed: Vec<usize> = first.to_vec();
    let mut depth = 0u8;
    let mut hidden_from: Option<u8> = None;
    for entry in rootlist {
        match entry {
            RootlistEntry::FolderStart { id, name } => {
                let open = !collapsed.contains(id);
                if hidden_from.is_none() {
                    rows.push(SideRow::Folder {
                        id: id.clone(),
                        name: name.clone(),
                        open,
                        depth,
                    });
                    if !open {
                        hidden_from = Some(depth);
                    }
                }
                depth = depth.saturating_add(1);
            }
            RootlistEntry::FolderEnd => {
                depth = depth.saturating_sub(1);
                if hidden_from == Some(depth) {
                    hidden_from = None;
                }
            }
            RootlistEntry::Playlist(uri) => {
                let Some(index) = playlists.iter().position(|p| p.uri == *uri) else {
                    continue;
                };
                if listed.contains(&index) {
                    continue;
                }
                listed.push(index);
                if hidden_from.is_none() {
                    rows.push(SideRow::Playlist { index, depth });
                }
            }
        }
    }
    for index in 0..playlists.len() {
        if !listed.contains(&index) {
            rows.push(SideRow::Playlist { index, depth: 0 });
        }
    }
    rows
}

/// The sidebar's material: #252525 at 76% over the page beside it, so a
/// coloured page shows through as a deep version of itself.
fn material(under: Color32) -> Color32 {
    under.lerp_to_gamma(Color32::from_rgb(0x25, 0x25, 0x25), 0.76)
}

/// The edge glow along the right side of the sidebar, matching mcloud's `#am-sidebar-glow`
/// (user.css line 206): a heavily blurred, saturated and brightened backdrop reaching
/// 160px inward from the sidebar edge, with a soft compression roll-off to avoid blow-out.
fn glow_color(tint: Color32) -> Option<Color32> {
    let r = tint.r() as f32 / 255.0;
    let g = tint.g() as f32 / 255.0;
    let b = tint.b() as f32 / 255.0;

    let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    if max_c - min_c < 0.04 && l < 0.25 {
        return None;
    }

    // Boost saturation (3.5x as in mcloud CSS backdrop-filter: saturate(3.5))
    let mut sr = l + (r - l) * 3.5;
    let mut sg = l + (g - l) * 3.5;
    let mut sb = l + (b - l) * 3.5;

    // Brightness (1.25x as in mcloud CSS backdrop-filter: brightness(1.25))
    sr *= 1.25;
    sg *= 1.25;
    sb *= 1.25;

    // Soft cap roll-off above 0.4 up to ~0.66 (matching #am-glow-cap tableValues)
    let cap = |v: f32| -> f32 {
        let v = v.clamp(0.0, 3.0);
        if v <= 0.4 {
            v
        } else {
            0.4 + (v - 0.4).atan() * (0.26 / (std::f32::consts::PI / 2.0))
        }
    };

    let cr = (cap(sr).clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (cap(sg).clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (cap(sb).clamp(0.0, 1.0) * 255.0).round() as u8;

    Some(Color32::from_rgb(cr, cg, cb))
}

fn paint_sidebar_glow(
    painter: &egui::Painter,
    edge_x: f32,
    top_y: f32,
    bottom_y: f32,
    tint: Color32,
) {
    let Some(glow_rgb) = glow_color(tint) else {
        return;
    };

    let mut mesh = egui::Mesh::default();
    // 160px inward matching linear-gradient(90deg, transparent calc(var(--am-side) - 160px), #0007 calc(var(--am-side) - 70px), #000 calc(var(--am-side) - 1px), transparent var(--am-side))
    let xs = [edge_x - 160.0, edge_x - 70.0, edge_x - 1.0, edge_x];
    let x_mult = [0.0, 0.44, 1.0, 0.0];
    let ys = [top_y, top_y + 160.0, top_y + 320.0, top_y + 540.0, bottom_y];
    let y_mult = [1.0, 1.0, 0.75, 0.30, 0.05];

    let r = glow_rgb.r();
    let g = glow_rgb.g();
    let b = glow_rgb.b();
    const PEAK_ALPHA: f32 = 85.0;

    for (iy, &y) in ys.iter().enumerate() {
        for (ix, &x) in xs.iter().enumerate() {
            let a = (PEAK_ALPHA * x_mult[ix] * y_mult[iy]).clamp(0.0, 255.0) as u8;
            let col = Color32::from_rgba_unmultiplied(r, g, b, a);
            mesh.colored_vertex(pos2(x, y), col);
        }
    }

    let cols = xs.len() as u32;
    for iy in 0..(ys.len() - 1) {
        let row_start = iy as u32 * cols;
        let next_row = row_start + cols;
        for ix in 0..(xs.len() - 1) {
            let tl = row_start + ix as u32;
            let tr = tl + 1;
            let bl = next_row + ix as u32;
            let br = bl + 1;
            mesh.add_triangle(tl, tr, br);
            mesh.add_triangle(tl, br, bl);
        }
    }

    painter.add(egui::Shape::mesh(mesh));
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let tint = super::super::page_tint(app).or_else(|| app.now_playing_tint());
    let under = tint.unwrap_or(app.palette.window);
    // The artist page tints the whole window (mcloud's `am-tinted`): the
    // sidebar is its colour at 34% in #161616, 85% over the page, and has
    // no glow.
    let page = app.page().clone();
    let (fill, tint) = match &page {
        Page::Artist(id) => {
            let (top, _) = super::artist::page_colours(app, id);
            let mixed = top.lerp_to_gamma(Color32::from_rgb(0x16, 0x16, 0x16), 0.66);
            (mixed.lerp_to_gamma(top, 0.15), None)
        }
        _ => (material(under), tint),
    };
    let beside = if app.show_queue_panel || app.show_lyrics_panel || app.show_friends_panel {
        theme::SIDE_PANEL_MIN_WIDTH
    } else {
        0.0
    };
    // Spotifast's sidebar panel, at mcloud's width: the one dragged to,
    // else 15.5% of the window, 220 to 300pt (user.css `--am-side`).
    let chosen = app
        .settings
        .mcloud_sidebar_width
        .unwrap_or_else(|| default_width(ui.ctx().content_rect().width()));
    let fit = super::super::yielding_panel(
        ui.ctx(),
        "sidebar",
        super::super::SIDEBAR_MIN_WIDTH..=600.0,
        chosen,
        ui.available_width() - super::super::topbar::least_width(ui.ctx()) - beside,
    );
    let response = egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(chosen)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(Frame::new().fill(fill).inner_margin(Margin {
            left: SIDE_PADDING as i8,
            right: SIDE_PADDING as i8,
            top: TOP as i8,
            bottom: 12,
        }))
        .show(ui, |ui| {
            let area = ui.max_rect();
            let edge_x = area.right() + SIDE_PADDING;
            let top_y = area.top() - TOP;
            let bottom_y = area.bottom() + 12.0;

            if let Some(t) = tint {
                paint_sidebar_glow(ui.painter(), edge_x, top_y, bottom_y, t);
            }

            // A hairline along the right edge, as macOS draws it.
            ui.painter().vline(
                edge_x - 0.5,
                top_y..=bottom_y,
                Stroke::new(1.0, Color32::from_white_alpha(10)),
            );
            let list = area.with_max_y(area.bottom() - ACCOUNT_HEIGHT);
            let scroll_anim_id = egui::Id::new("mcloud-sidebar-collapse-anim");
            let expanded_id = egui::Id::new("mcloud-playlists-expanded");
            let anim = ui
                .ctx()
                .data(|d| d.get_temp::<CollapseScrollAnim>(scroll_anim_id));
            let mut forced_scroll_y = None;
            if let Some(anim) = anim {
                let now = ui.ctx().input(|i| i.time);
                let elapsed = (now - anim.started_at) as f32;
                if anim.duration <= 0.0 || elapsed >= anim.duration {
                    forced_scroll_y = Some(anim.target_y);
                    ui.ctx().data_mut(|d| {
                        d.remove::<CollapseScrollAnim>(scroll_anim_id);
                        d.insert_temp(expanded_id, false);
                    });
                } else {
                    let t = ease_out_cubic(elapsed / anim.duration);
                    forced_scroll_y = Some(anim.start_y + (anim.target_y - anim.start_y) * t);
                    ui.ctx().request_repaint();
                }
            }
            let mut sticky_heading_y = None;
            let scroll_out = ui
                .scope_builder(UiBuilder::new().max_rect(list), |ui| {
                    let mut scroll = egui::ScrollArea::vertical()
                        .id_salt("mcloud-sidebar")
                        .auto_shrink(false)
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                        );
                    if let Some(y) = forced_scroll_y {
                        scroll = scroll.vertical_scroll_offset(y);
                    }
                    scroll.show(ui, |ui| contents(app, ui, &mut sticky_heading_y))
                })
                .inner;
            let scroll_y = scroll_out.state.offset.y;
            let viewport_h = scroll_out.inner_rect.height();
            let content_h = scroll_out.content_size.y;
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("mcloud-sidebar-scroll-metrics"),
                    (scroll_y, viewport_h),
                );
            });

            // Sticky "Playlists" header with "Show Less" when expanded and scrolled past the heading.
            let expanded = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(expanded_id))
                .unwrap_or(false);
            if expanded
                && let Some(heading_top) = sticky_heading_y
                && heading_top < list.top()
            {
                let sticky_rect =
                    Rect::from_min_size(pos2(list.left(), list.top()), vec2(list.width(), 28.0));
                ui.painter().rect_filled(sticky_rect, 0.0, fill);
                ui.painter().hline(
                    sticky_rect.x_range(),
                    sticky_rect.bottom() - 0.5,
                    Stroke::new(1.0, Color32::from_white_alpha(15)),
                );
                let add = gettext(app.locale, "Create a playlist");
                let show_less = gettext(app.locale, "Show Less");
                let (less_clicked, create_clicked) = paint_heading_in_rect(
                    ui,
                    sticky_rect,
                    "mcloud-sticky-playlists-heading",
                    &gettext(app.locale, "Playlists"),
                    Some((Some(show_less.as_ref()), add.as_ref())),
                );
                if less_clicked {
                    trigger_show_less(ui.ctx(), scroll_y, viewport_h);
                }
                if create_clicked {
                    start_inline_playlist_creation(ui.ctx(), app.locale);
                }
            }

            // Item #25: When playlists continue below the viewport, fade the bottom 16px
            // and show a subtle 1px hairline; at the end of the list, remove both.
            if scroll_y + viewport_h < content_h - 1.0 {
                let fade = Rect::from_min_max(
                    pos2(area.left(), (list.bottom() - 16.0).max(list.top())),
                    pos2(area.right(), list.bottom()),
                );
                crate::ui::widgets::paint_vertical_gradient(ui, fade, Color32::TRANSPARENT, fill);
                ui.painter().hline(
                    area.x_range(),
                    list.bottom() - 0.5,
                    Stroke::new(1.0, Color32::from_white_alpha(18)),
                );
            }

            let account = Rect::from_min_max(pos2(area.left(), list.bottom()), area.max);
            ui.scope_builder(UiBuilder::new().max_rect(account), |ui| {
                ui.set_clip_rect(account.intersect(ui.clip_rect()));
                account_row(app, ui);
            });
        });
    // Only a drag sets a width of its own: the default follows the window.
    let width = response.response.rect.width();
    if (width - chosen).abs() > 1.0 && super::super::panel_dragged(ui.ctx(), "sidebar") {
        app.settings.mcloud_sidebar_width = Some(width);
        app.actions.push(Action::SettingsChanged);
    }
}

/// mcloud's sidebar width for a window: `clamp(220px, 15.5vw, 300px)`.
fn default_width(window: f32) -> f32 {
    (window * 0.155).clamp(220.0, 300.0)
}

fn contents(app: &mut App, ui: &mut egui::Ui, sticky_heading_y: &mut Option<f32>) {
    let locale = app.locale;
    let page = app.page().clone();
    ui.spacing_mut().item_spacing.y = 0.0;

    if row(
        ui,
        Leading::Icon(Icon::Search),
        &gettext(locale, "Search"),
        page.is_search(),
        false,
    )
    .clicked()
    {
        app.actions.push(Action::FocusSearch);
    }
    if row(
        ui,
        Leading::Icon(Icon::McloudHome),
        &gettext(locale, "Home"),
        page == Page::Home,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Home));
    }
    if row(
        ui,
        Leading::Icon(Icon::McloudBrowse),
        &gettext(locale, "Browse"),
        page == Page::Browse,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Browse));
    }

    heading(ui, &gettext(locale, "Library"), None, &mut None);
    for (icon, label, target) in [
        (
            Icon::McloudClock,
            gettext(locale, "Recently Added"),
            Page::RecentlyAdded,
        ),
        (
            Icon::McloudArtist,
            gettext(locale, "Artists"),
            Page::Artists,
        ),
        (Icon::McloudAlbum, gettext(locale, "Albums"), Page::Albums),
        (
            Icon::McloudMusic,
            gettext(locale, "Songs"),
            Page::LikedSongs,
        ),
    ] {
        if row(ui, Leading::Icon(icon), &label, page == target, false).clicked() {
            app.actions.push(Action::Open(target));
        }
    }

    playlists(app, ui, &page, sticky_heading_y);
    ui.add_space(20.0);
}

fn trigger_show_less(ctx: &egui::Context, scroll_y: f32, viewport_h: f32) {
    let expanded_id = egui::Id::new("mcloud-playlists-expanded");
    let scroll_anim_id = egui::Id::new("mcloud-sidebar-collapse-anim");
    let target_y = 0.0_f32;
    let reduce_motion = crate::motion::reduced(ctx);
    if !reduce_motion && scroll_y > target_y + 16.0 && viewport_h > 0.0 {
        let max_distance = 2.0 * viewport_h;
        let start_y = scroll_y.min(target_y + max_distance);
        let distance = (start_y - target_y).max(0.0);
        let duration = 0.30 + 0.20 * (distance / max_distance).clamp(0.0, 1.0);
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| {
            d.insert_temp(
                scroll_anim_id,
                CollapseScrollAnim {
                    start_y,
                    target_y,
                    started_at: now,
                    duration,
                },
            );
        });
        ctx.request_repaint();
    } else {
        ctx.data_mut(|d| {
            d.remove::<CollapseScrollAnim>(scroll_anim_id);
            d.insert_temp(expanded_id, false);
        });
    }
}

fn start_inline_playlist_creation(ctx: &egui::Context, locale: crate::i18n::Locale) {
    let inline_id = egui::Id::new("mcloud-inline-create-playlist");
    let default_name = gettext(locale, "New Playlist").into_owned();
    ctx.data_mut(|d| {
        d.insert_temp(inline_id, default_name);
        d.insert_temp(inline_id.with("focus"), true);
    });
}

/// Pinned playlists first, in pin order, then the most recently played,
/// as Spotify's own sidebar sorted by Recents; "Show All" lists the rest.
fn playlists(app: &mut App, ui: &mut egui::Ui, page: &Page, sticky_heading_y: &mut Option<f32>) {
    let locale = app.locale;
    let expanded_id = egui::Id::new("mcloud-playlists-expanded");
    let mut expanded = ui
        .data(|data| data.get_temp::<bool>(expanded_id))
        .unwrap_or(false);

    let add = gettext(locale, "Create a playlist");
    let show_less = gettext(locale, "Show Less");
    let (less, create) = heading(
        ui,
        &gettext(locale, "Playlists"),
        Some((expanded.then_some(show_less.as_ref()), add.as_ref())),
        sticky_heading_y,
    );
    if less {
        let (scroll_y, viewport_h) = ui
            .ctx()
            .data(|d| d.get_temp::<(f32, f32)>(egui::Id::new("mcloud-sidebar-scroll-metrics")))
            .unwrap_or((0.0, 0.0));
        trigger_show_less(ui.ctx(), scroll_y, viewport_h);
        expanded = ui
            .data(|data| data.get_temp::<bool>(expanded_id))
            .unwrap_or(false);
    }
    if create {
        start_inline_playlist_creation(ui.ctx(), locale);
    }

    let all_playlists = gettext(locale, "All Playlists");
    if row(
        ui,
        Leading::Icon(Icon::McloudGrid),
        &all_playlists,
        *page == Page::AllPlaylists,
        false,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::AllPlaylists));
    }

    // Item #23: Inline playlist creation row at the top of the playlists list.
    let inline_id = egui::Id::new("mcloud-inline-create-playlist");
    let inline_draft = ui.data(|d| d.get_temp::<String>(inline_id));
    if let Some(mut draft) = inline_draft {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(ROW_RADIUS),
            Color32::from_white_alpha(20),
        );
        let lead = Rect::from_min_size(
            pos2(rect.left() + 14.0, rect.center().y - ART_SIZE / 2.0),
            Vec2::splat(ART_SIZE),
        );
        ui.painter()
            .rect_filled(lead, CornerRadius::same(4), Color32::from_white_alpha(20));
        theme::paint_icon(ui, Icon::McloudMusic, lead, 14.0, QUIET);
        let field_rect = Rect::from_min_max(
            pos2(lead.right() + 8.0, rect.top() + 2.0),
            pos2(rect.right() - 8.0, rect.bottom() - 2.0),
        );
        let edit_id = inline_id.with("edit");
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(field_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let response = child.add(
            egui::TextEdit::singleline(&mut draft)
                .id(edit_id)
                .font(theme::medium(13.0))
                .text_color(TEXT)
                .frame(egui::Frame::NONE)
                .desired_width(field_rect.width()),
        );
        let ask_focus = ui
            .data(|d| d.get_temp::<bool>(inline_id.with("focus")))
            .unwrap_or(false);
        if ask_focus {
            response.request_focus();
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit_id) {
                let end = egui::text::CCursor::new(draft.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        end,
                    )));
                state.store(ui.ctx(), edit_id);
            }
            ui.data_mut(|d| d.remove::<bool>(inline_id.with("focus")));
        }
        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if esc {
            ui.data_mut(|d| d.remove::<String>(inline_id));
        } else if enter || response.lost_focus() {
            let trimmed = draft.trim().to_string();
            ui.data_mut(|d| d.remove::<String>(inline_id));
            if !trimmed.is_empty() {
                app.actions.push(Action::CreatePlaylist {
                    name: trimmed,
                    public: false,
                    add_uris: Vec::new(),
                });
            }
        } else {
            ui.data_mut(|d| d.insert_temp(inline_id, draft));
        }
    }

    let Some(all) = app.library.playlists.get() else {
        return;
    };

    // Item #22: Recents polling schedule (0.1s, 0.4s, 1.5s, 5s, 15s) on playing context
    // change, and deferring background reorders while pointer/focus is inside the list.
    let playing = app.playing_context_uri();
    let sync_id = egui::Id::new("mcloud-sidebar-recents-sync");
    let list_active_id = egui::Id::new("mcloud-sidebar-list-active");
    let list_active = ui
        .data(|d| d.get_temp::<bool>(list_active_id))
        .unwrap_or(false);
    let now = ui.ctx().input(|i| i.time);
    let mut sync = ui
        .data(|d| d.get_temp::<RecentsSyncState>(sync_id))
        .unwrap_or_else(|| RecentsSyncState {
            last_playing: playing.clone(),
            changed_at: None,
            displayed_recents: app.recent_contexts.clone(),
        });
    let playing_changed = sync.last_playing != playing;
    if playing_changed {
        sync.last_playing = playing.clone();
        sync.changed_at = playing.as_ref().map(|_| now);
        sync.displayed_recents = app.recent_contexts.clone();
    } else if !list_active {
        sync.displayed_recents = app.recent_contexts.clone();
    }
    if let (Some(uri), Some(started)) = (playing.as_deref(), sync.changed_at) {
        let is_pinned = app.settings.pinned_contexts.iter().any(|p| p == uri);
        let is_top = sync.displayed_recents.first().is_some_and(|r| r == uri);
        let is_playlist = all.iter().any(|p| p.uri == uri);
        if is_playlist && !is_pinned && !is_top {
            let elapsed = (now - started).max(0.0);
            if elapsed >= RECENTS_POLL_DELAYS[0] {
                if !list_active {
                    sync.displayed_recents.retain(|r| r != uri);
                    sync.displayed_recents.insert(0, uri.to_string());
                    sync.changed_at = None;
                }
            } else if let Some(&next_delay) = RECENTS_POLL_DELAYS.iter().find(|&&d| d > elapsed) {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        (next_delay - elapsed).max(0.02),
                    ));
            }
        } else {
            sync.changed_at = None;
        }
    }
    ui.data_mut(|d| d.insert_temp(sync_id, sync.clone()));

    let collapsed_order = playlist_order(
        all,
        &app.settings.pinned_contexts,
        &sync.displayed_recents,
        false,
    );
    let collapsed_count = collapsed_order.len();
    let more = !expanded && collapsed_count < all.len();
    let rows = if expanded {
        expanded_rows(all, &app.rootlist, &app.collapsed_folders, &collapsed_order)
    } else {
        collapsed_order
            .iter()
            .map(|&index| SideRow::Playlist { index, depth: 0 })
            .collect()
    };

    let focus_new_id = egui::Id::new("mcloud-playlists-focus-first-new");
    let focus_first_new = ui
        .data(|d| d.get_temp::<bool>(focus_new_id))
        .unwrap_or(false);
    if focus_first_new {
        ui.data_mut(|d| d.remove::<bool>(focus_new_id));
    }

    let reduce_motion = crate::motion::reduced(ui.ctx());
    let art_px = app.art_px(ART_SIZE);
    let mut open = None;
    let mut any_hovered_or_focused = false;
    let mut toggle = None;
    for (slot, side_row) in rows.iter().enumerate() {
        let key = match side_row {
            SideRow::Playlist { index, .. } => all[*index].uri.as_str(),
            SideRow::Folder { id, .. } => id.as_str(),
        };
        let slot_id = egui::Id::new(("mcloud-sidebar-slot", key));
        let prev_slot = ui.data(|d| d.get_temp::<f32>(slot_id));
        ui.data_mut(|d| d.insert_temp(slot_id, slot as f32));
        let target_y = slot as f32 * ROW_HEIGHT;
        let anim_y = if reduce_motion {
            target_y
        } else if prev_slot.is_some() {
            ui.ctx().animate_value_with_time(
                slot_id.with("reorder-y"),
                target_y,
                crate::motion::seconds(ui.ctx(), 0.30),
            )
        } else {
            // New row drops 8px over 200ms.
            let drop = ui.ctx().animate_value_with_time(
                slot_id.with("entry-drop"),
                0.0,
                crate::motion::seconds(ui.ctx(), 0.20),
            );
            target_y + drop
        };
        if prev_slot.is_none() && !reduce_motion {
            ui.ctx()
                .data_mut(|d| d.insert_temp(slot_id.with("entry-drop"), -8.0_f32));
        }
        let y_shift = (anim_y - target_y).clamp(-64.0, 64.0);
        let response = match side_row {
            SideRow::Playlist { index, depth } => {
                let playlist = &all[*index];
                let image = pick_image(&playlist.images, art_px);
                let active = matches!(page, Page::Playlist(id) if *id == playlist.id);
                let end = if playing.as_deref() == Some(playlist.uri.as_str()) {
                    RowEnd::Playing
                } else if app.settings.pinned_contexts.contains(&playlist.uri) {
                    RowEnd::Pinned
                } else {
                    RowEnd::Nothing
                };
                let response = row_shifted(
                    ui,
                    Leading::Art(image, app.backend.art()),
                    &playlist.name,
                    active,
                    end,
                    *depth,
                    y_shift,
                );
                if response.clicked() {
                    open = Some(Page::Playlist(playlist.id.clone()));
                }
                response
            }
            SideRow::Folder {
                id,
                name,
                open: folder_open,
                depth,
            } => {
                let fallback = gettext(locale, "Folder");
                let label = if name.is_empty() {
                    fallback.as_ref()
                } else {
                    name.as_str()
                };
                let response = row_shifted(
                    ui,
                    Leading::Folder,
                    label,
                    false,
                    RowEnd::Folder { open: *folder_open },
                    *depth,
                    y_shift,
                );
                if response.clicked() {
                    toggle = Some(id.clone());
                }
                response
            }
        };
        if focus_first_new && slot == collapsed_count {
            response.request_focus();
        }
        if response.hovered() || response.has_focus() {
            any_hovered_or_focused = true;
        }
    }
    if let Some(id) = toggle {
        app.actions.push(Action::ToggleLibraryFolder(id));
    }
    ui.data_mut(|d| d.insert_temp(list_active_id, any_hovered_or_focused));

    if more {
        let show_all = row(
            ui,
            Leading::Chevron,
            &gettext(locale, "Show All"),
            false,
            false,
        );
        if show_all.clicked() {
            expanded = true;
            // The keyboard moves on to the first new row; a click stays put,
            // so no focus ring appears under the pointer.
            if !show_all.clicked_by(egui::PointerButton::Primary) {
                ui.data_mut(|d| d.insert_temp(focus_new_id, true));
            }
        }
    }
    if let Some(page) = open {
        app.actions.push(Action::Open(page));
    }
    ui.data_mut(|data| data.insert_temp(expanded_id, expanded));
}

/// The indices of the playlists to list, in order: pinned ones, then the
/// rest by Recents, as Spotify sorts them: the recently played first, then
/// the others in library order. Unless `all` is asked for, only
/// [`RECENT_PLAYLISTS`] follow the pinned ones.
fn playlist_order(
    playlists: &[Playlist],
    pinned: &[String],
    recent: &[String],
    all: bool,
) -> Vec<usize> {
    let position = |uri: &str| playlists.iter().position(|playlist| playlist.uri == uri);
    let mut order: Vec<usize> = Vec::new();
    for index in pinned.iter().filter_map(|uri| position(uri)) {
        if !order.contains(&index) {
            order.push(index);
        }
    }
    let shown = if all {
        playlists.len()
    } else {
        (order.len() + RECENT_PLAYLISTS).min(playlists.len())
    };
    let by_recents = recent
        .iter()
        .filter_map(|uri| position(uri))
        .chain(0..playlists.len());
    for index in by_recents {
        if order.len() == shown {
            break;
        }
        if !order.contains(&index) {
            order.push(index);
        }
    }
    order
}

/// A section heading. With `buttons`, it carries "Show Less" (when given)
/// and a "+" on its right; returns whether each was clicked.
fn heading(
    ui: &mut egui::Ui,
    label: &str,
    buttons: Option<(Option<&str>, &str)>,
    top_out: &mut Option<f32>,
) -> (bool, bool) {
    ui.add_space(16.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 13.0), Sense::hover());
    *top_out = Some(rect.top());
    ui.add_space(4.0);
    paint_heading_in_rect(ui, rect, "heading", label, buttons)
}

fn paint_heading_in_rect(
    ui: &mut egui::Ui,
    rect: Rect,
    id_salt: &str,
    label: &str,
    buttons: Option<(Option<&str>, &str)>,
) -> (bool, bool) {
    ui.painter().text(
        pos2(rect.left() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::semibold(11.0),
        HEADING,
    );
    let Some((less, add)) = buttons else {
        return (false, false);
    };
    // The "+" sits over the pin column, 5 points from the edge.
    let plus = Rect::from_center_size(
        pos2(rect.right() - 5.0 - 11.0, rect.center().y + 1.0),
        Vec2::splat(22.0),
    );
    let plus_response = ui.interact(plus, ui.id().with((id_salt, "add", label)), Sense::click());
    plus_response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, add));
    if plus_response.hovered() {
        ui.painter()
            .rect_filled(plus, CornerRadius::same(6), Color32::from_white_alpha(20));
    }
    theme::paint_icon(
        ui,
        Icon::McloudPlus,
        plus,
        15.0,
        if plus_response.hovered() { TEXT } else { QUIET },
    );
    let mut less_clicked = false;
    if let Some(less) = less {
        let galley = ui
            .painter()
            .layout_no_wrap(less.to_owned(), theme::semibold(11.0), QUIET);
        let pill = Rect::from_min_size(
            pos2(
                plus.left() - 4.0 - galley.size().x - 16.0,
                rect.center().y - 10.0,
            ),
            vec2(galley.size().x + 16.0, 20.0),
        );
        let response = ui.interact(pill, ui.id().with((id_salt, "less", label)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, less));
        let (fill, color) = if response.hovered() {
            (Color32::from_white_alpha(31), TEXT)
        } else {
            (Color32::from_white_alpha(15), QUIET)
        };
        ui.painter().rect_filled(pill, CornerRadius::same(10), fill);
        ui.painter().galley(
            pos2(pill.left() + 8.0, pill.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        less_clicked = response.clicked();
    }
    (less_clicked, plus_response.on_hover_text(add).clicked())
}

/// A navigation or playlist row. `playing` marks the playlist that is
/// playing with a speaker at the row's end.
fn row(
    ui: &mut egui::Ui,
    leading: Leading<'_>,
    label: &str,
    active: bool,
    playing: bool,
) -> egui::Response {
    let end = if playing {
        RowEnd::Playing
    } else {
        RowEnd::Nothing
    };
    row_shifted(ui, leading, label, active, end, 0, 0.0)
}

fn row_shifted(
    ui: &mut egui::Ui,
    leading: Leading<'_>,
    label: &str,
    active: bool,
    end: RowEnd,
    depth: u8,
    y_shift: f32,
) -> egui::Response {
    let (alloc_rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let rect = alloc_rect.translate(vec2(0.0, y_shift));
    if ui.is_rect_visible(rect) {
        let fill = if active {
            Color32::from_white_alpha(31)
        } else if response.hovered() {
            Color32::from_white_alpha(13)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
        // Playlist rows sit two points further left, so their artwork's
        // centre lines up with the icons above.
        let (lead_left, lead_size) = match leading {
            Leading::Art(..) | Leading::Folder => (rect.left() + 14.0, ART_SIZE),
            _ => (rect.left() + ROW_PADDING, ICON_SIZE),
        };
        let lead_left = lead_left + f32::from(depth) * 12.0;
        let lead = Rect::from_min_size(
            pos2(lead_left, rect.center().y - lead_size / 2.0),
            Vec2::splat(lead_size),
        );
        let mut color = TEXT;
        match leading {
            Leading::Icon(icon) => {
                icon.image(accent(), ICON_SIZE).paint_at(ui, lead);
            }
            Leading::Art(url, art) => {
                ui.painter().rect_filled(
                    lead,
                    CornerRadius::same(4),
                    Color32::from_white_alpha(20),
                );
                crate::ui::widgets::paint_cover(
                    ui,
                    &crate::theme::Palette::mcloud(),
                    url,
                    lead,
                    4.0,
                    Icon::McloudMusic,
                    Some(art),
                );
            }
            Leading::Chevron => {
                color = QUIET;
                theme::paint_icon(ui, Icon::McloudChevron, lead, 12.0, QUIET);
            }
            Leading::Folder => {
                ui.painter().rect_filled(
                    lead,
                    CornerRadius::same(4),
                    Color32::from_white_alpha(20),
                );
                theme::paint_icon(ui, Icon::McloudFolder, lead, 14.0, accent());
            }
        }
        let font = if active {
            theme::semibold(13.0)
        } else {
            theme::medium(13.0)
        };
        let gap = if matches!(leading, Leading::Art(..) | Leading::Folder) {
            8.0
        } else {
            ICON_GAP
        };
        let text_left = lead.right() + gap;
        let trailing = if end == RowEnd::Nothing {
            ROW_PADDING
        } else {
            14.0 + 8.0 + ROW_PADDING
        };
        let galley = crate::ui::widgets::ellipsized(
            ui,
            label,
            font,
            color,
            (rect.right() - trailing - text_left).max(0.0),
            1,
        );
        ui.painter().galley(
            pos2(text_left, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        let end_rect = Rect::from_center_size(
            pos2(rect.right() - ROW_PADDING - 7.0, rect.center().y),
            Vec2::splat(14.0),
        );
        match end {
            RowEnd::Nothing => {}
            RowEnd::Playing => {
                theme::paint_icon(ui, Icon::McloudVolume2, end_rect, 14.0, accent());
            }
            RowEnd::Pinned => theme::paint_icon(ui, Icon::Pin, end_rect, 12.0, QUIET),
            RowEnd::Folder { open } => {
                let angle = if open {
                    std::f32::consts::FRAC_PI_2
                } else {
                    0.0
                };
                Icon::McloudChevron
                    .image(QUIET, 11.0)
                    .rotate(angle, Vec2::splat(0.5))
                    .paint_at(
                        ui,
                        Rect::from_center_size(end_rect.center(), Vec2::splat(11.0)),
                    );
            }
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, label)
    });
    theme::focus_ring(ui, &response);
    response
}

/// mcloud's accent: its palette is fixed.
fn accent() -> Color32 {
    crate::theme::Palette::mcloud().accent
}

/// The account in the bottom-left corner, as in Apple Music: the chip opens
/// Settings, where the account card is, and the button beside it shows
/// what friends are listening to.
fn account_row(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let (name, avatar) = app
        .user
        .as_ref()
        .map(|user| {
            (
                user.name().to_string(),
                pick_image(&user.images, app.art_px(28.0)).map(str::to_string),
            )
        })
        .unwrap_or_default();
    let area = ui.max_rect();
    let line = Rect::from_min_max(pos2(area.left(), area.bottom() - 40.0), area.max);
    let friends = Rect::from_center_size(
        pos2(line.right() - 15.0, line.center().y),
        Vec2::splat(30.0),
    );
    let news = friends.translate(vec2(-32.0, 0.0));
    let chip = Rect::from_min_max(line.min, pos2(news.left() - 2.0, line.bottom()));

    let response = ui.interact(chip, ui.id().with("mcloud-account"), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(chip, CornerRadius::same(8), Color32::from_white_alpha(13));
    }
    let face = Rect::from_center_size(
        pos2(chip.left() + 8.0 + 14.0, chip.center().y),
        Vec2::splat(28.0),
    );
    match avatar.as_deref() {
        Some(url) => crate::ui::widgets::paint_cover(
            ui,
            &palette,
            Some(url),
            face,
            14.0,
            Icon::User,
            Some(app.backend.art()),
        ),
        None => {
            crate::ui::widgets::paint_vertical_gradient(
                ui,
                face,
                Color32::from_rgb(0xa6, 0xb4, 0xd6),
                Color32::from_rgb(0x6d, 0x7b, 0xa0),
            );
            let initial = name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            ui.painter().text(
                face.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                theme::semibold(11.0),
                Color32::WHITE,
            );
        }
    }
    let galley = crate::ui::widgets::ellipsized(
        ui,
        &name,
        theme::semibold(13.0),
        TEXT,
        (chip.right() - 8.0 - face.right() - 10.0).max(0.0),
        1,
    );
    ui.painter().galley(
        pos2(face.right() + 10.0, chip.center().y - galley.size().y / 2.0),
        galley,
        TEXT,
    );
    if response.clicked() {
        app.actions.push(Action::Open(Page::Settings));
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &name)
    });
    theme::focus_ring(ui, &response);

    // What's New: Spotify's feed of releases from the artists followed.
    let label = gettext(locale, "What's New");
    let response = ui.interact(news, ui.id().with("mcloud-whats-new"), Sense::click());
    let here = *app.page() == Page::NewReleases;
    let response = response.on_hover_text(label.as_ref());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, here, label.as_ref())
    });
    if response.hovered() {
        ui.painter()
            .rect_filled(news, CornerRadius::same(8), Color32::from_white_alpha(13));
    }
    let color = if here {
        accent()
    } else if response.hovered() {
        TEXT
    } else {
        Color32::from_rgb(0xa9, 0xa9, 0xa9)
    };
    theme::paint_icon(ui, Icon::McloudBell, news, 18.0, color);
    theme::focus_ring(ui, &response);
    if response.clicked() {
        app.actions.push(Action::Open(Page::NewReleases));
    }

    let label = gettext(locale, "Listening activity");
    let response = ui.interact(friends, ui.id().with("mcloud-friends"), Sense::click());
    let is_down = response.is_pointer_button_down_on();
    let clicked = response.clicked();
    let active = (app.show_friends_panel ^ clicked) || is_down;
    let response = response.on_hover_text(label.as_ref());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, label.as_ref())
    });
    let color = if active {
        accent()
    } else if response.hovered() {
        TEXT
    } else {
        Color32::from_rgb(0xa9, 0xa9, 0xa9)
    };
    if active || response.hovered() {
        ui.painter().rect_filled(
            friends,
            CornerRadius::same(8),
            Color32::from_white_alpha(if active { 20 } else { 13 }),
        );
    }
    theme::paint_icon(ui, Icon::McloudFriends, friends, 18.0, color);
    if clicked {
        app.actions.push(Action::ToggleFriendsPanel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist(uri: &str) -> Playlist {
        serde_json::from_value(serde_json::json!({
            "id": uri.rsplit(':').next().unwrap(),
            "name": uri,
            "uri": uri,
        }))
        .unwrap()
    }

    /// The opened list keeps the short list's rows first, then follows the
    /// account's order: a rolled-up folder hides its playlists, and one the
    /// short list already shows is not listed twice.
    #[test]
    fn the_opened_list_follows_the_rootlist_with_its_folders() {
        use crate::player::RootlistEntry;
        let playlist = |uri: &str| Playlist {
            uri: uri.into(),
            ..Default::default()
        };
        let all = [playlist("a"), playlist("b"), playlist("c"), playlist("d")];
        let rootlist = [
            RootlistEntry::FolderStart {
                id: "open".into(),
                name: "Open".into(),
            },
            RootlistEntry::Playlist("a".into()),
            RootlistEntry::Playlist("b".into()),
            RootlistEntry::FolderEnd,
            RootlistEntry::FolderStart {
                id: "shut".into(),
                name: "Shut".into(),
            },
            RootlistEntry::Playlist("c".into()),
            RootlistEntry::FolderEnd,
        ];
        let rows = expanded_rows(&all, &rootlist, &["shut".into()], &[1]);
        let folder = |id: &str, open| SideRow::Folder {
            id: id.into(),
            name: if open { "Open" } else { "Shut" }.into(),
            open,
            depth: 0,
        };
        assert_eq!(
            rows,
            [
                SideRow::Playlist { index: 1, depth: 0 },
                folder("open", true),
                SideRow::Playlist { index: 0, depth: 1 },
                folder("shut", false),
                // Not in the rootlist yet: waits at the end.
                SideRow::Playlist { index: 3, depth: 0 },
            ]
        );
    }

    /// Like mcloud's, the sidebar is 15.5% of the window, 220 to 300pt.
    #[test]
    fn the_sidebar_follows_the_window_between_220_and_300() {
        assert_eq!(default_width(1000.0), 220.0);
        assert!((default_width(1600.0) - 248.0).abs() < 0.01);
        assert_eq!(default_width(2560.0), 300.0);
    }

    #[test]
    fn pinned_lead_then_recents_up_to_the_limit_then_the_rest_when_asked() {
        let playlists: Vec<Playlist> = (0..10)
            .map(|n| playlist(&format!("spotify:playlist:p{n}")))
            .collect();
        let uri = |n: usize| format!("spotify:playlist:p{n}");
        let pinned = vec![uri(7), "spotify:album:gone".into()];
        // The pinned one played recently too; it is not listed twice.
        let recent: Vec<String> = [7, 2, 4, 9, 1, 3, 5, 8].map(uri).to_vec();

        assert_eq!(
            playlist_order(&playlists, &pinned, &recent, false),
            vec![7, 2, 4, 9, 1, 3]
        );
        assert_eq!(
            playlist_order(&playlists, &pinned, &recent, true),
            vec![7, 2, 4, 9, 1, 3, 5, 8, 0, 6]
        );
        // Nothing played yet: the library's own order fills the list.
        assert_eq!(
            playlist_order(&playlists, &pinned, &[], false),
            vec![7, 0, 1, 2, 3, 4]
        );
        assert_eq!(playlist_order(&playlists[..2], &[], &[], false), vec![0, 1]);
    }

    #[test]
    fn ease_out_cubic_and_recents_poll_schedule_match_spec() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!((ease_out_cubic(0.5) - 0.875).abs() < 1e-5);
        assert_eq!(RECENTS_POLL_DELAYS, [0.1, 0.4, 1.5, 5.0, 15.0]);
    }
}

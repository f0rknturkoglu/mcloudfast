//! mcloud's look: Apple Music's layout drawn over Spotifast's own state.
//!
//! Settings → Appearance → Interface style chooses it
//! ([`crate::settings::Interface`]). Each view here stands in for one of
//! Spotifast's and emits the same [`crate::model::Action`]s, so playback,
//! navigation and the library behave the same in either look.

pub mod artist;
pub mod glass;
pub mod library;
pub mod lyrics;
pub mod pages;
pub mod player;
pub mod search;
pub mod sidebar;
pub mod topbar;

/// Whether mcloud draws this frame, for the widgets it shares with
/// Spotifast's look (track rows, table headers), which have no `App` to ask.
/// [`crate::ui::show`] records it at the start of every frame.
pub fn active(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(active_id()))
        .unwrap_or(false)
}

pub(super) fn set_active(ctx: &egui::Context, active: bool) {
    ctx.data_mut(|data| data.insert_temp(active_id(), active));
}

pub fn glass_effect(ctx: &egui::Context) -> crate::settings::GlassEffect {
    ctx.data(|data| data.get_temp::<crate::settings::GlassEffect>(glass_effect_id()))
        .unwrap_or_default()
}

pub(super) fn set_glass_effect(ctx: &egui::Context, effect: crate::settings::GlassEffect) {
    ctx.data_mut(|data| data.insert_temp(glass_effect_id(), effect));
}

fn active_id() -> egui::Id {
    egui::Id::new("mcloud-active")
}

fn glass_effect_id() -> egui::Id {
    egui::Id::new("mcloud-glass-effect")
}

/// mcloud's page padding (user.css `--am-pad`): 2.6% of the window's width,
/// 28 to 48pt.
pub fn page_padding(ctx: &egui::Context) -> f32 {
    padding_for(ctx.content_rect().width())
}

fn padding_for(window: f32) -> f32 {
    (window * 0.026).clamp(28.0, 48.0)
}

/// A column heading as mcloud writes it: "TITLE" as "Title".
pub fn sentence_case(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn headings_read_in_sentence_case() {
        assert_eq!(super::sentence_case("DATE ADDED"), "Date added");
        assert_eq!(super::sentence_case("TITLE"), "Title");
        assert_eq!(super::sentence_case(""), "");
    }

    #[test]
    fn pages_pad_by_the_window_width() {
        assert_eq!(super::padding_for(800.0), 28.0);
        assert!((super::padding_for(1400.0) - 36.4).abs() < 0.01);
        assert_eq!(super::padding_for(1916.0), 48.0);
    }
}

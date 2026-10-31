//! mcloud's look: Apple Music's layout drawn over Spotifast's own state.
//!
//! Settings → Appearance → Interface style chooses it
//! ([`crate::settings::Interface`]). Each view here stands in for one of
//! Spotifast's and emits the same [`crate::model::Action`]s, so playback,
//! navigation and the library behave the same in either look.

pub mod artist;
pub mod glass;
pub mod pages;
pub mod player;
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

fn active_id() -> egui::Id {
    egui::Id::new("mcloud-active")
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
}

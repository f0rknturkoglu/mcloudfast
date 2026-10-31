//! mcloud's look: Apple Music's layout drawn over Spotifast's own state.
//!
//! Settings → Appearance → Interface style chooses it
//! ([`crate::settings::Interface`]). Each view here stands in for one of
//! Spotifast's and emits the same [`crate::model::Action`]s, so playback,
//! navigation and the library behave the same in either look.

pub mod glass;
pub mod player;
pub mod sidebar;

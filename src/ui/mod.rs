//! UI shell: desktop background, window chrome, and the menu bar. Nothing
//! in here holds domain state on its own — it reads/writes through the
//! `os` and `app` layers.

pub mod desktop;
pub mod fullscreen;
pub mod menu_bar;
pub mod window_frame;

use leptos::prelude::*;

/// Whether the start menu popover is open. Shared between the menu bar
/// (which toggles it) and the desktop (which closes it on outside click).
#[derive(Clone, Copy)]
pub struct StartMenuOpen(pub RwSignal<bool>);

pub fn provide() {
    leptos::prelude::provide_context(StartMenuOpen(RwSignal::new(false)));
    fullscreen::provide();
}

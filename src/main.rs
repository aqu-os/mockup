//! Aquos UI mockup — a playground for trying out desktop UI ideas before
//! they land in the real OS.
//!
//! - `os/` holds logic that will eventually belong to the OS itself —
//!   `os::kernel` owns process/window/settings/filesystem state behind a
//!   `syscall()` boundary, the rest is window-manager policy on top —
//!   kept UI-framework-light.
//! - `app/` holds the "programs" that can run on the mockup desktop.
//! - `ui/` is the shell that renders it all: wallpaper, window chrome,
//!   menu bar.

mod app;
mod os;
mod ui;

use leptos::prelude::*;
use ui::desktop::Desktop;

/// Wires up the OS-level and UI-level context before rendering the
/// desktop. `provide_context` only reaches components rendered below it in
/// the same reactive scope, so this has to live inside the mounted tree
/// rather than in `main`.
#[component]
fn Root() -> impl IntoView {
    os::kernel::init();
    let workspaces = os::provide();
    ui::provide();
    let installed = app::InstalledApps::new();
    provide_context(installed);
    provide_context(app::Apps::new(workspaces, installed));

    view! { <Desktop /> }
}

fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Warn);

    mount_to_body(Root);
}

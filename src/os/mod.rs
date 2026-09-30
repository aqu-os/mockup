//! Logic that belongs to the OS itself rather than to any particular app or
//! UI. `kernel` owns process/window-identity/settings/filesystem state and
//! is reached only through its `syscall()` — everything else here (window
//! managers, workspaces) is policy built on top of that, deliberately kept
//! out of the kernel. `ui/` and `app/` are the only modules allowed to
//! depend on this one for rendering — nothing in here should know about
//! window chrome, styling, or specific apps.

pub mod kernel;
pub mod managers;
pub mod window_manager;
pub mod workspace;

use workspace::WorkspaceManager;

/// Installs the OS-level singletons into the Leptos context tree and
/// returns them so callers can wire up higher layers without a second
/// context lookup. Call this once, near the root of the app.
pub fn provide() -> WorkspaceManager {
    let workspaces = WorkspaceManager::new();
    leptos::prelude::provide_context(workspaces);
    workspaces
}

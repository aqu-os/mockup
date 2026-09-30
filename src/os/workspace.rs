//! Workspaces: independent desktops, each with its own set of windows and
//! its own trio of window-manager services (floating, tiling,
//! infinite-scroll — see [`super::managers`]). A workspace keeps all
//! three alive at once and just switches which one is *active*, so
//! resizing/reordering state isn't lost when the user flips a workspace's
//! layout back and forth.
//!
//! Window identity, focus and stacking live in the kernel; a `Workspace`
//! only tags kernel calls with its own id and layers the (non-kernel)
//! layout-manager policy on top.

use leptos::prelude::*;

use super::kernel::{self, Pid, SizeConstraints, WindowId, WindowMeta};
use super::managers::{AnyWindowManager, FloatingManager, InfiniteScrollManager, TilingManager};
use super::window_manager::WindowManager;

pub use super::kernel::WorkspaceId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayoutKind {
    /// Freely positioned, draggable windows.
    Floating,
    /// Windows arranged in a single row that always fits the screen.
    Tiling,
    /// Windows placed side by side along a horizontally scrolling strip.
    InfiniteScroll,
}

impl LayoutKind {
    pub const ALL: &'static [LayoutKind] =
        &[LayoutKind::Floating, LayoutKind::Tiling, LayoutKind::InfiniteScroll];

    pub fn label(&self) -> &'static str {
        match self {
            LayoutKind::Floating => "Floating",
            LayoutKind::Tiling => "Tiling",
            LayoutKind::InfiniteScroll => "Infinite scroll",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LayoutKind::Floating => "\u{25F0}",
            LayoutKind::Tiling => "\u{25A6}",
            LayoutKind::InfiniteScroll => "\u{21C9}",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: &'static str,
    pub layout: RwSignal<LayoutKind>,
    floating: FloatingManager,
    tiling: TilingManager,
    infinite_scroll: InfiniteScrollManager,
}

impl Workspace {
    fn new(id: WorkspaceId, name: &'static str, layout: LayoutKind) -> Self {
        Self {
            id,
            name,
            layout: RwSignal::new(layout),
            floating: FloatingManager::new(),
            tiling: TilingManager::new(),
            infinite_scroll: InfiniteScrollManager::new(),
        }
    }

    /// The window-manager service currently governing this workspace's
    /// layout.
    pub fn active_manager(&self) -> AnyWindowManager {
        match self.layout.get() {
            LayoutKind::Floating => AnyWindowManager::Floating(self.floating),
            LayoutKind::Tiling => AnyWindowManager::Tiling(self.tiling),
            LayoutKind::InfiniteScroll => AnyWindowManager::InfiniteScroll(self.infinite_scroll),
        }
    }

    /// Opens a window: registers its identity with the kernel, then hands
    /// it to *all three* managers so each has layout state ready the
    /// moment the user switches to it, not just the currently-active one.
    pub fn open_window(
        &self,
        owner: Pid,
        title: impl Into<String>,
        icon: &'static str,
        preferred_size: (f64, f64),
        constraints: SizeConstraints,
    ) -> WindowId {
        let id = kernel::open_window(owner, self.id, title, icon, constraints);
        self.floating.on_open(id, preferred_size, constraints);
        self.tiling.on_open(id, preferred_size, constraints);
        self.infinite_scroll.on_open(id, preferred_size, constraints);
        id
    }

    pub fn close_window(&self, id: WindowId) {
        kernel::close_window(id);
        self.floating.on_close(id);
        self.tiling.on_close(id);
        self.infinite_scroll.on_close(id);
    }

    /// Focuses a window in this workspace.
    pub fn focus(&self, id: WindowId) {
        kernel::focus_window(id);
    }

    /// The currently focused window in this workspace, if any.
    pub fn focused(&self) -> Option<WindowId> {
        kernel::focused_window(self.id)
    }

    /// A window's fixed metadata (title, icon, ...), read once.
    pub fn window_meta(&self, id: WindowId) -> Option<WindowMeta> {
        kernel::window_meta(id)
    }
}

/// Registry of workspaces plus which one is currently active. Switching
/// workspaces swaps out the entire set of visible windows.
#[derive(Clone, Copy)]
pub struct WorkspaceManager {
    workspaces: RwSignal<Vec<Workspace>>,
    active: RwSignal<WorkspaceId>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let workspaces = vec![
            Workspace::new(1, "1", LayoutKind::Floating),
            Workspace::new(2, "2", LayoutKind::Tiling),
            Workspace::new(3, "3", LayoutKind::InfiniteScroll),
        ];
        Self {
            workspaces: RwSignal::new(workspaces),
            active: RwSignal::new(1),
        }
    }

    /// Reactive read access to all workspaces.
    pub fn all(&self) -> RwSignal<Vec<Workspace>> {
        self.workspaces
    }

    pub fn active_id(&self) -> RwSignal<WorkspaceId> {
        self.active
    }

    pub fn set_active(&self, id: WorkspaceId) {
        self.active.set(id);
    }

    pub fn workspace(&self, id: WorkspaceId) -> Option<Workspace> {
        self.workspaces.get().into_iter().find(|w| w.id == id)
    }

    pub fn active_workspace(&self) -> Workspace {
        let id = self.active.get();
        self.workspace(id).expect("active workspace always exists")
    }
}

//! The three window-manager services. Each is a small, self-contained
//! layout strategy implementing [`super::window_manager::WindowManager`].

pub mod floating;
pub mod infinite_scroll;
pub mod tiling;

pub use floating::FloatingManager;
pub use infinite_scroll::InfiniteScrollManager;
pub use tiling::TilingManager;

use super::kernel::{SizeConstraints, WindowId};
use super::window_manager::{Placement, ResizeEdge, WindowAction, WindowActionKind, WindowManager};

/// Dispatches to whichever concrete manager is active for a workspace.
/// A plain enum rather than `dyn WindowManager`: Leptos's reactive
/// primitives require `Send + Sync`, which a trait object would need to
/// carry explicitly, and there are only ever three implementations, so
/// static dispatch is both simpler and cheaper.
#[derive(Clone, Copy)]
pub enum AnyWindowManager {
    Floating(FloatingManager),
    Tiling(TilingManager),
    InfiniteScroll(InfiniteScrollManager),
}

impl WindowManager for AnyWindowManager {
    fn on_open(&self, id: WindowId, preferred_size: (f64, f64), constraints: SizeConstraints) {
        match self {
            Self::Floating(m) => m.on_open(id, preferred_size, constraints),
            Self::Tiling(m) => m.on_open(id, preferred_size, constraints),
            Self::InfiniteScroll(m) => m.on_open(id, preferred_size, constraints),
        }
    }

    fn on_close(&self, id: WindowId) {
        match self {
            Self::Floating(m) => m.on_close(id),
            Self::Tiling(m) => m.on_close(id),
            Self::InfiniteScroll(m) => m.on_close(id),
        }
    }

    fn display_order(&self) -> Vec<WindowId> {
        match self {
            Self::Floating(m) => m.display_order(),
            Self::Tiling(m) => m.display_order(),
            Self::InfiniteScroll(m) => m.display_order(),
        }
    }

    fn placement(&self, id: WindowId) -> Option<Placement> {
        match self {
            Self::Floating(m) => m.placement(id),
            Self::Tiling(m) => m.placement(id),
            Self::InfiniteScroll(m) => m.placement(id),
        }
    }

    fn is_maximized(&self, id: WindowId) -> bool {
        match self {
            Self::Floating(m) => m.is_maximized(id),
            Self::Tiling(m) => m.is_maximized(id),
            Self::InfiniteScroll(m) => m.is_maximized(id),
        }
    }

    fn is_minimized(&self, id: WindowId) -> bool {
        match self {
            Self::Floating(m) => m.is_minimized(id),
            Self::Tiling(m) => m.is_minimized(id),
            Self::InfiniteScroll(m) => m.is_minimized(id),
        }
    }

    fn set_minimized(&self, id: WindowId, minimized: bool) {
        match self {
            Self::Floating(m) => m.set_minimized(id, minimized),
            Self::Tiling(m) => m.set_minimized(id, minimized),
            Self::InfiniteScroll(m) => m.set_minimized(id, minimized),
        }
    }

    fn supports_drag(&self) -> bool {
        match self {
            Self::Floating(m) => m.supports_drag(),
            Self::Tiling(m) => m.supports_drag(),
            Self::InfiniteScroll(m) => m.supports_drag(),
        }
    }

    fn drag_to(&self, id: WindowId, x: f64, y: f64) {
        match self {
            Self::Floating(m) => m.drag_to(id, x, y),
            Self::Tiling(m) => m.drag_to(id, x, y),
            Self::InfiniteScroll(m) => m.drag_to(id, x, y),
        }
    }

    fn resizable_edges(&self, id: WindowId) -> &'static [ResizeEdge] {
        match self {
            Self::Floating(m) => m.resizable_edges(id),
            Self::Tiling(m) => m.resizable_edges(id),
            Self::InfiniteScroll(m) => m.resizable_edges(id),
        }
    }

    fn resize_by(&self, id: WindowId, edge: ResizeEdge, dx: f64, dy: f64) {
        match self {
            Self::Floating(m) => m.resize_by(id, edge, dx, dy),
            Self::Tiling(m) => m.resize_by(id, edge, dx, dy),
            Self::InfiniteScroll(m) => m.resize_by(id, edge, dx, dy),
        }
    }

    fn actions(&self, id: WindowId) -> Vec<WindowAction> {
        match self {
            Self::Floating(m) => m.actions(id),
            Self::Tiling(m) => m.actions(id),
            Self::InfiniteScroll(m) => m.actions(id),
        }
    }

    fn perform(&self, id: WindowId, action: WindowActionKind) {
        match self {
            Self::Floating(m) => m.perform(id, action),
            Self::Tiling(m) => m.perform(id, action),
            Self::InfiniteScroll(m) => m.perform(id, action),
        }
    }
}

//! The `WindowManager` trait: the contract every layout service (floating,
//! tiling, infinite-scroll — see [`super::managers`]) implements. A window
//! manager doesn't own window identity or focus (that's the kernel's job,
//! via [`super::kernel`]) — it only decides where a window sits on screen
//! and which actions the user can currently perform on it.

use super::kernel::{SizeConstraints, WindowId};

/// Where and how big a window is, in a shape its layout strategy finds
/// natural. `ui::window_frame` translates whichever variant it gets into
/// concrete CSS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// Freely positioned, pixel-exact — floating windows.
    Absolute {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        z: i32,
    },
    /// A flex item taking a proportional share of a row that always fits
    /// the screen — tiling windows. `grow` is a relative weight, not a
    /// percentage; the row's flexbox normalizes it. `height` is never
    /// independently user-resizable — it fills the row, unless the app
    /// has a locked aspect ratio (which a "fill the row" height could
    /// violate), in which case it's derived from width instead.
    Flex { grow: f64, height: Extent },
    /// A card in a horizontally scrolling strip — infinite-scroll windows.
    /// `width` is user-resizable (a fixed pixel size, or — while
    /// maximized — filling the available width); `height` isn't, same
    /// fill-or-ratio-derived rule as tiling's.
    Scroll { width: Extent, height: Extent },
}

/// A window's extent along one axis: either a concrete pixel size, or
/// filling whatever space is available along that axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extent {
    Fixed(f64),
    /// Fills the available space along this axis.
    Fill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowActionKind {
    Minimize,
    Maximize,
    Restore,
    WidenStep,
    NarrowStep,
    MoveLeft,
    MoveRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowAction {
    pub kind: WindowActionKind,
    pub label: &'static str,
    pub icon: &'static str,
}

/// Which edge (or corner, as a pair of edges) of a window's box a resize
/// drag is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeEdge {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl ResizeEdge {
    /// All eight handles — floating windows get the full set (corner drag
    /// plus edges).
    pub const ALL: [ResizeEdge; 8] = [
        ResizeEdge::N,
        ResizeEdge::S,
        ResizeEdge::E,
        ResizeEdge::W,
        ResizeEdge::NE,
        ResizeEdge::NW,
        ResizeEdge::SE,
        ResizeEdge::SW,
    ];

    /// Just the two horizontal edges — what tiling and infinite-scroll
    /// windows get. Their height is managed by the row/strip (or derived
    /// from an aspect ratio), never independently draggable.
    pub const WIDTH_ONLY: [ResizeEdge; 2] = [ResizeEdge::E, ResizeEdge::W];

    pub fn affects_west(self) -> bool {
        matches!(self, ResizeEdge::W | ResizeEdge::NW | ResizeEdge::SW)
    }

    pub fn affects_east(self) -> bool {
        matches!(self, ResizeEdge::E | ResizeEdge::NE | ResizeEdge::SE)
    }

    pub fn affects_north(self) -> bool {
        matches!(self, ResizeEdge::N | ResizeEdge::NW | ResizeEdge::NE)
    }

    pub fn affects_south(self) -> bool {
        matches!(self, ResizeEdge::S | ResizeEdge::SW | ResizeEdge::SE)
    }

    /// The CSS cursor to show while hovering or dragging this handle.
    pub fn cursor(self) -> &'static str {
        match self {
            ResizeEdge::N | ResizeEdge::S => "ns-resize",
            ResizeEdge::E | ResizeEdge::W => "ew-resize",
            ResizeEdge::NE | ResizeEdge::SW => "nesw-resize",
            ResizeEdge::NW | ResizeEdge::SE => "nwse-resize",
        }
    }

    /// Lowercase class-name suffix used to position this handle in
    /// `ui::window_frame`'s CSS (e.g. `window__resize-handle--ne`).
    pub fn css_class(self) -> &'static str {
        match self {
            ResizeEdge::N => "n",
            ResizeEdge::S => "s",
            ResizeEdge::E => "e",
            ResizeEdge::W => "w",
            ResizeEdge::NE => "ne",
            ResizeEdge::NW => "nw",
            ResizeEdge::SE => "se",
            ResizeEdge::SW => "sw",
        }
    }
}

impl WindowActionKind {
    fn describe(self) -> (&'static str, &'static str) {
        match self {
            WindowActionKind::Minimize => ("Minimize", "_"),
            WindowActionKind::Maximize => ("Maximize", "\u{1F5D6}"),
            WindowActionKind::Restore => ("Restore", "\u{1F5D7}"),
            WindowActionKind::WidenStep => ("Widen", "+"),
            WindowActionKind::NarrowStep => ("Narrow", "\u{2212}"),
            WindowActionKind::MoveLeft => ("Move left", "\u{2190}"),
            WindowActionKind::MoveRight => ("Move right", "\u{2192}"),
        }
    }

    pub fn action(self) -> WindowAction {
        let (label, icon) = self.describe();
        WindowAction {
            kind: self,
            label,
            icon,
        }
    }
}

/// A layout strategy for a workspace's windows. Every method is scoped to
/// a single window by id; implementations keep their own internal
/// per-window state (e.g. a floating window's dragged position, a tiling
/// window's width share) alongside the shared metadata the kernel owns.
pub trait WindowManager {
    /// Registers a newly opened window so this manager can start tracking
    /// layout state for it. `preferred_size` is the app's requested
    /// initial size; `constraints` are its min/max bounds.
    fn on_open(&self, id: WindowId, preferred_size: (f64, f64), constraints: SizeConstraints);

    /// Drops a closed window's layout state.
    fn on_close(&self, id: WindowId);

    /// Where the window currently is. `None` only when `id` isn't known to
    /// this manager (e.g. mid-close).
    fn placement(&self, id: WindowId) -> Option<Placement>;

    /// This manager's left-to-right (or, for floating, arbitrary) display
    /// order of its windows. Tiling and infinite-scroll lay out children
    /// via flexbox, where visual order follows DOM order — so "move
    /// left"/"move right" have to change *this*, not just a window's
    /// size, for the reorder to actually be visible. The UI layer sorts
    /// its window list by this before rendering.
    fn display_order(&self) -> Vec<WindowId>;

    /// Whether this window is currently in its manager's "maximized"
    /// state. Always `false` for managers without a maximize action.
    fn is_maximized(&self, id: WindowId) -> bool {
        let _ = id;
        false
    }

    /// Whether this window is currently minimized. A minimized window
    /// stays mounted (so its app keeps running with its state intact) but
    /// is hidden from the desktop and represented only by its taskbar
    /// entry — the UI layer reads this to decide whether to render it.
    fn is_minimized(&self, id: WindowId) -> bool {
        let _ = id;
        false
    }

    /// Minimizes or restores this window. Every manager supports this
    /// (unlike maximize, which only some do) since it's just visibility,
    /// not a layout concern — restoring puts it back exactly where its
    /// layout state already says it belongs.
    fn set_minimized(&self, id: WindowId, minimized: bool);

    /// Whether windows under this manager can be freely dragged by their
    /// title bar.
    fn supports_drag(&self) -> bool {
        false
    }

    /// Called continuously while the user drags a window's title bar.
    /// No-ops under managers that don't support dragging.
    fn drag_to(&self, id: WindowId, x: f64, y: f64) {
        let (_, _, _) = (id, x, y);
    }

    /// Which resize handles this window currently offers — empty for a
    /// manager (or a window mid-maximize) that doesn't support resizing
    /// right now. The UI layer only renders handles for edges this
    /// returns, so a manager that only exposes [`ResizeEdge::WIDTH_ONLY`]
    /// never gets a vertical or diagonal drag.
    fn resizable_edges(&self, id: WindowId) -> &'static [ResizeEdge] {
        let _ = id;
        &[]
    }

    /// Called continuously while the user drags a resize handle on
    /// `edge`. `dx`/`dy` are the pointer's movement since the previous
    /// call (not an absolute position — a resize also has to know which
    /// side is the fixed anchor that doesn't move). No-ops under managers
    /// that don't support resizing.
    fn resize_by(&self, id: WindowId, edge: ResizeEdge, dx: f64, dy: f64) {
        let (_, _, _, _) = (id, edge, dx, dy);
    }

    /// The actions currently available for this window (varies with its
    /// state — e.g. a maximized window offers "Restore" instead of
    /// "Maximize", the leftmost tile has no "Move left").
    fn actions(&self, id: WindowId) -> Vec<WindowAction>;

    /// Applies one of the actions returned by [`Self::actions`].
    fn perform(&self, id: WindowId, action: WindowActionKind);
}

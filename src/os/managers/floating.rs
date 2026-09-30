//! Floating: freely positioned, draggable windows that can be maximized.

use leptos::prelude::*;

use crate::os::kernel::{self as kernel, ResizeAxis, SizeConstraints, WindowId};
use crate::os::window_manager::{Placement, ResizeEdge, WindowAction, WindowActionKind, WindowManager};

#[derive(Clone, Debug, PartialEq)]
struct FloatingState {
    id: WindowId,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    maximized: bool,
    minimized: bool,
    /// The rect to return to on restore, saved when maximizing.
    restore: Option<(f64, f64, f64, f64)>,
    constraints: SizeConstraints,
}

#[derive(Clone, Copy)]
pub struct FloatingManager {
    windows: RwSignal<Vec<FloatingState>>,
}

impl FloatingManager {
    pub fn new() -> Self {
        Self {
            windows: RwSignal::new(Vec::new()),
        }
    }

    fn cascade_offset(&self) -> (f64, f64) {
        let n = self.windows.get_untracked().len() as f64;
        let offset = (n % 6.0) * 26.0;
        (140.0 + offset, 90.0 + offset)
    }
}

impl WindowManager for FloatingManager {
    fn on_open(&self, id: WindowId, preferred_size: (f64, f64), constraints: SizeConstraints) {
        let (x, y) = self.cascade_offset();
        let (width, height) = constraints.clamp_size(preferred_size.0, preferred_size.1, ResizeAxis::Width);
        self.windows.update(|w| {
            w.push(FloatingState {
                id,
                x,
                y,
                width,
                height,
                maximized: false,
                minimized: false,
                restore: None,
                constraints,
            })
        });
    }

    fn on_close(&self, id: WindowId) {
        self.windows.update(|w| w.retain(|s| s.id != id));
    }

    fn display_order(&self) -> Vec<WindowId> {
        // Floating positions windows absolutely, so DOM order doesn't
        // affect how they look — insertion order is as good as any.
        self.windows.get().iter().map(|s| s.id).collect()
    }

    fn placement(&self, id: WindowId) -> Option<Placement> {
        self.windows.get().into_iter().find(|s| s.id == id).map(|s| Placement::Absolute {
            x: s.x,
            y: s.y,
            width: s.width,
            height: s.height,
            z: kernel::window_stack_position(id),
        })
    }

    fn is_maximized(&self, id: WindowId) -> bool {
        self.windows.get().iter().find(|s| s.id == id).map(|s| s.maximized).unwrap_or(false)
    }

    fn is_minimized(&self, id: WindowId) -> bool {
        self.windows.get().iter().find(|s| s.id == id).map(|s| s.minimized).unwrap_or(false)
    }

    fn set_minimized(&self, id: WindowId, minimized: bool) {
        self.windows.update(|w| {
            if let Some(s) = w.iter_mut().find(|s| s.id == id) {
                s.minimized = minimized;
            }
        });
    }

    fn supports_drag(&self) -> bool {
        true
    }

    fn drag_to(&self, id: WindowId, x: f64, y: f64) {
        self.windows.update(|w| {
            if let Some(s) = w.iter_mut().find(|s| s.id == id) {
                if !s.maximized {
                    s.x = x;
                    s.y = y;
                }
            }
        });
    }

    fn resizable_edges(&self, id: WindowId) -> &'static [ResizeEdge] {
        if self.is_maximized(id) {
            &[]
        } else {
            &ResizeEdge::ALL
        }
    }

    fn resize_by(&self, id: WindowId, edge: ResizeEdge, dx: f64, dy: f64) {
        self.windows.update(|w| {
            let Some(s) = w.iter_mut().find(|s| s.id == id) else {
                return;
            };
            if s.maximized {
                return;
            }
            let anchor_right = s.x + s.width;
            let anchor_bottom = s.y + s.height;

            let mut width = s.width;
            let mut height = s.height;
            if edge.affects_east() {
                width += dx;
            }
            if edge.affects_west() {
                width -= dx;
            }
            if edge.affects_south() {
                height += dy;
            }
            if edge.affects_north() {
                height -= dy;
            }

            let horizontal = edge.affects_east() || edge.affects_west();
            let vertical = edge.affects_north() || edge.affects_south();
            let axis = if vertical && (!horizontal || dy.abs() > dx.abs()) {
                ResizeAxis::Height
            } else {
                ResizeAxis::Width
            };
            let (width, height) = s.constraints.clamp_size(width, height, axis);

            s.width = width;
            s.height = height;
            if edge.affects_west() {
                s.x = anchor_right - width;
            }
            if edge.affects_north() {
                s.y = anchor_bottom - height;
            }
        });
    }

    fn actions(&self, id: WindowId) -> Vec<WindowAction> {
        let maximized = self.is_maximized(id);
        let maximize_kind = if maximized {
            WindowActionKind::Restore
        } else {
            WindowActionKind::Maximize
        };
        vec![WindowActionKind::Minimize.action(), maximize_kind.action()]
    }

    fn perform(&self, id: WindowId, action: WindowActionKind) {
        self.windows.update(|w| {
            let Some(s) = w.iter_mut().find(|s| s.id == id) else {
                return;
            };
            match action {
                WindowActionKind::Minimize => s.minimized = true,
                WindowActionKind::Maximize if !s.maximized => {
                    s.restore = Some((s.x, s.y, s.width, s.height));
                    s.maximized = true;
                }
                WindowActionKind::Restore if s.maximized => {
                    if let Some((x, y, width, height)) = s.restore.take() {
                        s.x = x;
                        s.y = y;
                        s.width = width;
                        s.height = height;
                    }
                    s.maximized = false;
                }
                _ => {}
            }
        });
    }
}

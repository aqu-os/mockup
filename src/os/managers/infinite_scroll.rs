//! Infinite scroll: windows placed side by side along a horizontally
//! panning strip. Each has an adjustable pixel width and can be
//! maximized — which fills the visible strip width while leaving its
//! neighbors in place, so the user can still scroll sideways to reach
//! them, rather than hiding everything else outright.

use leptos::prelude::*;

use crate::os::kernel::{self as kernel, os_category, Owner, ResizeAxis, SizeConstraints, WindowId};
use crate::os::window_manager::{Extent, Placement, ResizeEdge, WindowAction, WindowActionKind, WindowManager};

/// Settings-store key for the resize step, configurable from the Settings
/// app under Window Managers.
pub const STEP_KEY: &str = "scroll_step";
pub const DEFAULT_STEP: f64 = 60.0;
const MIN_WIDTH: f64 = 260.0;
const MAX_WIDTH: f64 = 900.0;

#[derive(Clone, Debug, PartialEq)]
struct ScrollState {
    id: WindowId,
    width: f64,
    /// Never independently user-resizable — fills the strip, like every
    /// window here always used to, unless an aspect ratio pins it to a
    /// value derived from width instead.
    height: Extent,
    maximized: bool,
    minimized: bool,
    constraints: SizeConstraints,
}

/// Vec order is left-to-right sequence along the strip.
#[derive(Clone, Copy)]
pub struct InfiniteScrollManager {
    windows: RwSignal<Vec<ScrollState>>,
}

impl InfiniteScrollManager {
    pub fn new() -> Self {
        Self {
            windows: RwSignal::new(Vec::new()),
        }
    }

    fn step(&self) -> f64 {
        kernel::get_setting_number(Owner::Os(os_category::WINDOW_MANAGERS), STEP_KEY, DEFAULT_STEP)
    }
}

impl WindowManager for InfiniteScrollManager {
    fn on_open(&self, id: WindowId, preferred_size: (f64, f64), constraints: SizeConstraints) {
        let width = constraints.clamp_width(preferred_size.0.max(MIN_WIDTH));
        // A locked aspect ratio can't be satisfied by "fill the strip" —
        // a width-derived height is required from the start. Otherwise,
        // always fill it — height is never independently resizable.
        let height = match constraints.aspect_ratio {
            Some(_) => Extent::Fixed(constraints.clamp_size(width, preferred_size.1, ResizeAxis::Width).1),
            None => Extent::Fill,
        };
        self.windows.update(|w| {
            w.push(ScrollState {
                id,
                width,
                height,
                maximized: false,
                minimized: false,
                constraints,
            })
        });
    }

    fn on_close(&self, id: WindowId) {
        self.windows.update(|w| w.retain(|s| s.id != id));
    }

    fn display_order(&self) -> Vec<WindowId> {
        self.windows.get().iter().map(|s| s.id).collect()
    }

    fn placement(&self, id: WindowId) -> Option<Placement> {
        self.windows.get().into_iter().find(|s| s.id == id).map(|s| {
            if s.maximized {
                Placement::Scroll { width: Extent::Fill, height: Extent::Fill }
            } else {
                Placement::Scroll { width: Extent::Fixed(s.width), height: s.height }
            }
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

    fn resizable_edges(&self, id: WindowId) -> &'static [ResizeEdge] {
        if self.is_maximized(id) {
            &[]
        } else {
            &ResizeEdge::WIDTH_ONLY
        }
    }

    fn resize_by(&self, id: WindowId, edge: ResizeEdge, dx: f64, _dy: f64) {
        self.windows.update(|w| {
            let Some(s) = w.iter_mut().find(|s| s.id == id) else {
                return;
            };
            if s.maximized {
                return;
            }
            let mut width = s.width;
            if edge.affects_east() {
                width += dx;
            }
            if edge.affects_west() {
                width -= dx;
            }
            // Only a locked aspect ratio ties height to width; otherwise
            // height stays exactly as it was (filling the strip).
            match s.constraints.aspect_ratio {
                Some(_) => {
                    let (width, height) = s.constraints.clamp_size(width.max(MIN_WIDTH), 0.0, ResizeAxis::Width);
                    s.width = width;
                    s.height = Extent::Fixed(height);
                }
                None => {
                    s.width = s.constraints.clamp_width(width.max(MIN_WIDTH));
                }
            }
        });
    }

    fn actions(&self, id: WindowId) -> Vec<WindowAction> {
        let windows = self.windows.get();
        let Some(idx) = windows.iter().position(|s| s.id == id) else {
            return Vec::new();
        };
        let n = windows.len();
        let state = &windows[idx];
        let mut actions = vec![WindowActionKind::Minimize.action()];

        actions.push(if state.maximized {
            WindowActionKind::Restore.action()
        } else {
            WindowActionKind::Maximize.action()
        });

        // Resizing and reordering only make sense while not filling the
        // strip's full width.
        if !state.maximized {
            if idx > 0 {
                actions.push(WindowActionKind::MoveLeft.action());
            }
            let min = state.constraints.min_width.max(MIN_WIDTH);
            if state.width > min {
                actions.push(WindowActionKind::NarrowStep.action());
            }
            let max = state.constraints.max_width.unwrap_or(MAX_WIDTH).min(MAX_WIDTH);
            if state.width < max {
                actions.push(WindowActionKind::WidenStep.action());
            }
            if idx + 1 < n {
                actions.push(WindowActionKind::MoveRight.action());
            }
        }

        actions
    }

    fn perform(&self, id: WindowId, action: WindowActionKind) {
        self.windows.update(|w| {
            let Some(idx) = w.iter().position(|s| s.id == id) else {
                return;
            };
            match action {
                WindowActionKind::Minimize => w[idx].minimized = true,
                WindowActionKind::Maximize => w[idx].maximized = true,
                WindowActionKind::Restore => w[idx].maximized = false,
                WindowActionKind::WidenStep if !w[idx].maximized => {
                    let max = w[idx].constraints.max_width.unwrap_or(MAX_WIDTH).min(MAX_WIDTH);
                    w[idx].width = (w[idx].width + self.step()).min(max);
                }
                WindowActionKind::NarrowStep if !w[idx].maximized => {
                    let min = w[idx].constraints.min_width.max(MIN_WIDTH);
                    w[idx].width = (w[idx].width - self.step()).max(min);
                }
                WindowActionKind::MoveLeft if idx > 0 => w.swap(idx, idx - 1),
                WindowActionKind::MoveRight if idx + 1 < w.len() => w.swap(idx, idx + 1),
                _ => {}
            }
        });
    }
}

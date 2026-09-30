//! Tiling: windows arranged in a single row that always fits the screen —
//! no manual positioning, but each window's width share can be adjusted in
//! steps, and windows can be reordered left/right.

use leptos::prelude::*;

use crate::os::kernel::{self as kernel, os_category, Owner, ResizeAxis, SizeConstraints, WindowId};
use crate::os::window_manager::{Extent, Placement, ResizeEdge, WindowAction, WindowActionKind, WindowManager};

/// Settings-store key for the resize step, configurable from the Settings
/// app under Window Managers.
pub const STEP_KEY: &str = "tiling_step";
pub const DEFAULT_STEP: f64 = 0.1;
const MIN_WEIGHT: f64 = 0.15;
const MAX_WEIGHT: f64 = 0.7;
const EPSILON: f64 = 1e-6;

/// Tiling doesn't track the row's actual on-screen pixel width — a
/// window's width lives here only as a proportional `weight`, turned into
/// real pixels by the flexbox layout in CSS. Converting a horizontal
/// resize drag into a weight delta, or deriving a width from an aspect
/// ratio, both need *some* assumed reference width to convert against.
/// This is that assumption: rough, but in the same spirit as
/// [`DEFAULT_STEP`] already being an arbitrary increment rather than a
/// physically exact one.
const REFERENCE_ROW_WIDTH: f64 = 1200.0;

#[derive(Clone, Debug, PartialEq)]
struct TilingState {
    id: WindowId,
    /// Relative width share. Not required to sum to 1 across all windows —
    /// it feeds a flexbox `flex-grow`, which normalizes it — but we keep
    /// it normalized anyway so [`MIN_WEIGHT`]/[`MAX_WEIGHT`] read as plain
    /// fractions.
    weight: f64,
    /// Unlike width, a tiling window's height is never independently
    /// user-resizable — it fills the row, like every window here always
    /// used to, unless an aspect ratio pins it to a value derived from
    /// width instead.
    height: Extent,
    minimized: bool,
    constraints: SizeConstraints,
}

/// Vec order is left-to-right display order.
#[derive(Clone, Copy)]
pub struct TilingManager {
    windows: RwSignal<Vec<TilingState>>,
}

impl TilingManager {
    pub fn new() -> Self {
        Self {
            windows: RwSignal::new(Vec::new()),
        }
    }

    fn step(&self) -> f64 {
        kernel::get_setting_number(Owner::Os(os_category::WINDOW_MANAGERS), STEP_KEY, DEFAULT_STEP)
    }

    /// Resets every window to an equal share. Kept deliberately simple:
    /// a window opening or closing reflows the whole row evenly rather
    /// than trying to preserve relative proportions.
    fn rebalance(windows: &mut [TilingState]) {
        if windows.is_empty() {
            return;
        }
        let equal = 1.0 / windows.len() as f64;
        for w in windows.iter_mut() {
            w.weight = equal;
        }
    }
}

impl WindowManager for TilingManager {
    fn on_open(&self, id: WindowId, preferred_size: (f64, f64), constraints: SizeConstraints) {
        // A locked aspect ratio can't be satisfied by "fill the row" — a
        // width-derived height is required from the start. Otherwise,
        // always fill the row — height is never independently resizable.
        let height = match constraints.aspect_ratio {
            Some(_) => Extent::Fixed(constraints.clamp_size(preferred_size.0, preferred_size.1, ResizeAxis::Width).1),
            None => Extent::Fill,
        };
        self.windows.update(|w| {
            w.push(TilingState {
                id,
                weight: 1.0,
                height,
                minimized: false,
                constraints,
            });
            Self::rebalance(w);
        });
    }

    fn on_close(&self, id: WindowId) {
        self.windows.update(|w| {
            w.retain(|s| s.id != id);
            Self::rebalance(w);
        });
    }

    fn display_order(&self) -> Vec<WindowId> {
        self.windows.get().iter().map(|s| s.id).collect()
    }

    fn placement(&self, id: WindowId) -> Option<Placement> {
        self.windows
            .get()
            .into_iter()
            .find(|s| s.id == id)
            .map(|s| Placement::Flex { grow: s.weight, height: s.height })
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
        let _ = id;
        &ResizeEdge::WIDTH_ONLY
    }

    fn resize_by(&self, id: WindowId, edge: ResizeEdge, dx: f64, _dy: f64) {
        self.windows.update(|w| {
            let Some(idx) = w.iter().position(|s| s.id == id) else {
                return;
            };
            let constraints = w[idx].constraints;
            let mut width_px = w[idx].weight * REFERENCE_ROW_WIDTH;
            if edge.affects_east() {
                width_px += dx;
            }
            if edge.affects_west() {
                width_px -= dx;
            }
            // Only a locked aspect ratio ties height to width; otherwise
            // height stays exactly as it was (filling the row).
            let width_px = match constraints.aspect_ratio {
                Some(_) => {
                    let (width_px, height) = constraints.clamp_size(width_px, 0.0, ResizeAxis::Width);
                    w[idx].height = Extent::Fixed(height);
                    width_px
                }
                None => constraints.clamp_width(width_px),
            };
            let weight_delta = width_px / REFERENCE_ROW_WIDTH - w[idx].weight;
            if weight_delta.abs() > EPSILON {
                resize(w, idx, weight_delta);
            }
        });
    }

    fn actions(&self, id: WindowId) -> Vec<WindowAction> {
        let windows = self.windows.get();
        let Some(idx) = windows.iter().position(|s| s.id == id) else {
            return Vec::new();
        };
        let n = windows.len();
        let weight = windows[idx].weight;
        let mut actions = vec![WindowActionKind::Minimize.action()];
        if idx > 0 {
            actions.push(WindowActionKind::MoveLeft.action());
        }
        if n > 1 && weight > MIN_WEIGHT + EPSILON {
            actions.push(WindowActionKind::NarrowStep.action());
        }
        if n > 1 && weight < MAX_WEIGHT - EPSILON {
            actions.push(WindowActionKind::WidenStep.action());
        }
        if idx + 1 < n {
            actions.push(WindowActionKind::MoveRight.action());
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
                WindowActionKind::WidenStep => resize(w, idx, self.step()),
                WindowActionKind::NarrowStep => resize(w, idx, -self.step()),
                WindowActionKind::MoveLeft if idx > 0 => w.swap(idx, idx - 1),
                WindowActionKind::MoveRight if idx + 1 < w.len() => w.swap(idx, idx + 1),
                _ => {}
            }
        });
    }
}

/// Grows or shrinks the window at `idx` by `delta`, taking (or giving back)
/// the difference from every other window proportionally to their current
/// share, so the row always stays balanced.
fn resize(windows: &mut [TilingState], idx: usize, delta: f64) {
    let n = windows.len();
    if n < 2 {
        return;
    }
    let current = windows[idx].weight;
    let target = (current + delta).clamp(MIN_WEIGHT, MAX_WEIGHT);
    let actual_delta = target - current;
    if actual_delta.abs() < EPSILON {
        return;
    }

    let others_total: f64 = windows.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, s)| s.weight).sum();
    if others_total <= EPSILON {
        return;
    }
    // Would shrinking the others below the floor to make room? Bail rather
    // than violate MIN_WEIGHT.
    if others_total - actual_delta < MIN_WEIGHT * (n - 1) as f64 - EPSILON {
        return;
    }

    for (i, w) in windows.iter_mut().enumerate() {
        if i == idx {
            w.weight = target;
        } else {
            let share = w.weight / others_total;
            w.weight -= actual_delta * share;
        }
    }
}

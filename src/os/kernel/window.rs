//! Kernel-private window state: identity, ownership, size constraints,
//! focus and stacking order — scoped per workspace. Deliberately knows
//! nothing about *where* a window sits on screen; that's computed by
//! whichever [`crate::os::window_manager`] service is active for the
//! workspace. Reached only through [`super::syscall`].

use std::collections::HashMap;

use leptos::prelude::*;

use super::process::Pid;

pub type WindowId = u32;
pub type WorkspaceId = u32;

/// Size limits an app reports for one of its windows. `max` is `None` when
/// the app has no upper bound (most apps).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeConstraints {
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: Option<f64>,
    pub max_height: Option<f64>,
    /// Locked width / height ratio, if the app needs one (e.g. a video
    /// player). Window managers keep a window's box at this ratio while
    /// it's being resized — but never while it's maximized, since a
    /// maximized window is explicitly filling its space rather than being
    /// sized by the user.
    pub aspect_ratio: Option<f64>,
}

/// Which dimension a resize interaction is primarily driven by — e.g.
/// dragging the right edge drives width. When a window has a locked
/// [`SizeConstraints::aspect_ratio`], the other dimension is derived from
/// whichever one this names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAxis {
    Width,
    Height,
}

impl SizeConstraints {
    pub fn clamp_width(&self, width: f64) -> f64 {
        let width = width.max(self.min_width);
        match self.max_width {
            Some(max) => width.min(max),
            None => width,
        }
    }

    pub fn clamp_height(&self, height: f64) -> f64 {
        let height = height.max(self.min_height);
        match self.max_height {
            Some(max) => height.min(max),
            None => height,
        }
    }

    /// Clamps a proposed `(width, height)` box to this window's min/max
    /// bounds and, if set, its locked aspect ratio. `axis` is which
    /// dimension the caller is actively driving; the other is derived from
    /// it to preserve the ratio, then both are clamped again in case
    /// deriving pushed either one out of its min/max bounds.
    pub fn clamp_size(&self, width: f64, height: f64, axis: ResizeAxis) -> (f64, f64) {
        let width = self.clamp_width(width);
        let height = self.clamp_height(height);
        let Some(ratio) = self.aspect_ratio else {
            return (width, height);
        };
        match axis {
            ResizeAxis::Width => {
                let height = self.clamp_height(width / ratio);
                let width = self.clamp_width(height * ratio);
                (width, height)
            }
            ResizeAxis::Height => {
                let width = self.clamp_width(height * ratio);
                let height = self.clamp_height(width / ratio);
                (width, height)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowMeta {
    pub id: WindowId,
    pub owner: Pid,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub icon: &'static str,
    pub constraints: SizeConstraints,
}

/// Tracks which windows exist, their metadata, and — per workspace — which
/// one is focused and their front-to-back stacking order. Geometry lives
/// elsewhere (in the active window-manager service); this is just the
/// shared bookkeeping every layout strategy needs regardless of how it
/// arranges windows on screen.
#[derive(Clone, Copy)]
pub(super) struct WindowState {
    windows: RwSignal<Vec<WindowMeta>>,
    next_id: RwSignal<WindowId>,
    /// Focused window per workspace.
    focused: RwSignal<HashMap<WorkspaceId, WindowId>>,
    /// Bottom-to-top stacking order per workspace; last element is on top.
    stacks: RwSignal<HashMap<WorkspaceId, Vec<WindowId>>>,
}

impl WindowState {
    pub(super) fn new() -> Self {
        Self {
            windows: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(1),
            focused: RwSignal::new(HashMap::new()),
            stacks: RwSignal::new(HashMap::new()),
        }
    }

    pub(super) fn open(
        &self,
        workspace_id: WorkspaceId,
        owner: Pid,
        title: String,
        icon: &'static str,
        constraints: SizeConstraints,
    ) -> WindowId {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        self.windows.update(|w| {
            w.push(WindowMeta {
                id,
                owner,
                workspace_id,
                title,
                icon,
                constraints,
            })
        });
        self.stacks.update(|s| s.entry(workspace_id).or_default().push(id));
        self.focused.update(|f| {
            f.insert(workspace_id, id);
        });
        id
    }

    /// Every window belongs to exactly one workspace, so callers scoped to
    /// a single window (close/focus/stack position) don't need to pass its
    /// workspace id separately — it's looked up here instead.
    fn workspace_of(&self, id: WindowId) -> Option<WorkspaceId> {
        self.windows.get_untracked().iter().find(|m| m.id == id).map(|m| m.workspace_id)
    }

    pub(super) fn close(&self, id: WindowId) {
        let Some(workspace_id) = self.workspace_of(id) else {
            return;
        };
        self.windows.update(|w| w.retain(|meta| meta.id != id));
        self.stacks.update(|s| {
            if let Some(stack) = s.get_mut(&workspace_id) {
                stack.retain(|&i| i != id);
            }
        });
        if self.focused.get_untracked().get(&workspace_id).copied() == Some(id) {
            let top = self.stacks.get_untracked().get(&workspace_id).and_then(|s| s.last().copied());
            self.focused.update(|f| match top {
                Some(t) => {
                    f.insert(workspace_id, t);
                }
                None => {
                    f.remove(&workspace_id);
                }
            });
        }
    }

    /// No-ops if `id` is already focused, so that repeated `pointerdown`s
    /// inside an already-focused window (every click bubbles into a focus
    /// call) don't keep touching the stack and forcing dependents to
    /// recompute.
    pub(super) fn focus(&self, id: WindowId) {
        let Some(workspace_id) = self.workspace_of(id) else {
            return;
        };
        if self.focused.get_untracked().get(&workspace_id).copied() == Some(id) {
            return;
        }
        self.stacks.update(|s| {
            let stack = s.entry(workspace_id).or_default();
            stack.retain(|&i| i != id);
            stack.push(id);
        });
        self.focused.update(|f| {
            f.insert(workspace_id, id);
        });
    }

    /// This window's position in its workspace's stack (0 = bottom). Used
    /// by the floating manager as a z-index.
    pub(super) fn stack_position(&self, id: WindowId) -> i32 {
        let Some(workspace_id) = self.workspace_of(id) else {
            return 0;
        };
        self.stacks
            .get()
            .get(&workspace_id)
            .and_then(|s| s.iter().position(|&i| i == id))
            .map(|p| p as i32)
            .unwrap_or(0)
    }

    /// A single window's metadata, read untracked — title/icon are fixed
    /// for a window's lifetime and are meant to be read exactly once, not
    /// re-rendered every time some other window's focus changes.
    pub(super) fn meta(&self, id: WindowId) -> Option<WindowMeta> {
        self.windows.get_untracked().into_iter().find(|m| m.id == id)
    }

    pub(super) fn focused(&self, workspace_id: WorkspaceId) -> Option<WindowId> {
        self.focused.get().get(&workspace_id).copied()
    }
}

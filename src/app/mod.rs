//! The app layer. Apps are the things a user launches — each one owns a
//! process and a window, both identity kept by [`crate::os::kernel`], and
//! knows how to render its own body. This module is the registry that ties
//! those two together and is the place to plug in new mockup apps.
//!
//! Rather than a closed enum of known apps, [`App`] is a trait — every app
//! is an independent struct implementing it, registered once in [`ALL`].
//! Adding an app means writing a new struct and adding it to that list, not
//! editing a central match statement (there isn't one).

pub mod app_store;
pub mod calculator;
pub mod contacts;
pub mod contacts_filter;
pub mod contacts_sort;
pub mod contacts_source;
pub mod file_explorer;
pub mod notes;
pub mod pipeline;
pub mod program_manager;
pub mod settings_manager;
pub mod source_inspector;
pub mod workbench;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

use leptos::prelude::*;

use crate::os::kernel::{
    self as kernel, AppRole, BlockId, Owner, Pid, SettingField, SettingValue, SizeConstraints, WindowId,
};
use crate::os::workspace::{WorkspaceId, WorkspaceManager};

/// One user-triggerable operation a `Source` app declares against its own
/// data — e.g. "add contact". Sinks discover these generically via
/// [`App::actions`] and render them as buttons/forms without knowing
/// anything about the source ahead of time, then call
/// [`App::invoke_action`] to run one.
pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub params: &'static [SettingField],
}

/// The contract every app implements. Each app is a separate zero-sized
/// struct (e.g. [`notes::NotesApp`]) — this trait is what lets the rest of
/// the system (launcher, taskbar, App Store, Settings) treat all of them
/// uniformly without knowing their concrete types.
pub trait App: Sync {
    /// A stable identifier used as this app's settings-storage namespace
    /// (see [`crate::os::kernel::Owner::App`]) — the closest thing this
    /// mockup has to an app's package id.
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn icon(&self) -> &'static str;

    /// System apps are installed by default and can't be uninstalled —
    /// they're essential to operating the desktop itself.
    fn is_system(&self) -> bool {
        false
    }

    /// Whether this app is installed out of the box versus only available
    /// from the App Store.
    fn installed_by_default(&self) -> bool {
        true
    }

    /// Whether only one instance of this app may run per workspace.
    /// Focuses the existing instance instead of spawning a new one when
    /// true.
    fn is_singleton(&self) -> bool {
        false
    }

    /// The size a new window should open at.
    fn preferred_size(&self) -> (f64, f64);

    /// How small/large this app is willing to let its window get. Window
    /// managers (floating, tiling, infinite-scroll) use this to clamp
    /// whatever size they'd otherwise compute.
    fn size_constraints(&self) -> SizeConstraints;

    /// The settings this app exposes — conceptually the manifest file it
    /// "installs" describing its configurable settings. The Settings app
    /// renders a form from this for any installed app without knowing
    /// anything about it ahead of time.
    fn settings_manifest(&self) -> &'static [SettingField] {
        &[]
    }

    /// This app's role in the source/manipulator/sink data model (see
    /// root `CLAUDE.md`), if it participates in one at all — most apps
    /// (Settings, Program Manager, ...) don't and stay `None`. The kernel
    /// gates data-block and pipeline syscalls on the role a process was
    /// spawned with (see `os::kernel::AppRole`); this is where that role
    /// comes from.
    fn role(&self) -> Option<AppRole> {
        None
    }

    /// `Source` apps only: the initial contents of the data block this
    /// app owns, as JSON. Read once, when the app's process first spawns.
    fn seed(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    /// `Source` apps only: a JSON descriptor of the block's shape —
    /// currently just field names for a generic viewer (like the source
    /// inspector) to use as table headers. Real typed/binary memory
    /// layout is deferred, per root `CLAUDE.md`.
    fn schema(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    /// `Source` apps only: the actions it exposes for sinks to surface
    /// generically (see [`Action`]).
    fn actions(&self) -> &'static [Action] {
        &[]
    }

    /// `Source` apps only: runs one of this app's declared [`actions`](App::actions)
    /// against the block it owns.
    fn invoke_action(
        &self,
        _block_id: BlockId,
        _owner: Pid,
        _action_id: &str,
        _params: &HashMap<&'static str, SettingValue>,
    ) {
    }

    /// `Manipulator` apps only: the parameters this manipulator exposes
    /// (e.g. which field to sort by), rendered generically wherever a
    /// pipeline chip for this manipulator is shown.
    fn param_schema(&self) -> &'static [SettingField] {
        &[]
    }

    /// `Manipulator` apps only: a pure transform from one JSON value to
    /// another — no write access to the source's block, just whatever it
    /// derives from the input. Defaults to the identity transform.
    fn apply(&self, input: serde_json::Value, _params: &HashMap<&'static str, SettingValue>) -> serde_json::Value {
        input
    }

    /// Renders this app's window body. Takes the pid of the process this
    /// window belongs to — needed by sources/sinks to find their own data
    /// block and pipeline attachments (a process can't just scan by app
    /// id, since a singleton could in principle have one instance per
    /// workspace); most apps ignore it.
    fn render(&self, pid: Pid) -> AnyView;
}

/// A handle to a registered app: a `'static` reference to its `dyn App`
/// trait object. Every app struct is a zero-sized singleton, so this is
/// cheap to copy around and compare — identity is decided by `id()`, not
/// pointer equality, so it stays stable regardless of how many places hold
/// a reference to the same app.
#[derive(Clone, Copy)]
pub struct AppRef(pub &'static dyn App);

impl PartialEq for AppRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl Eq for AppRef {}

impl Hash for AppRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state);
    }
}

impl fmt::Debug for AppRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AppRef").field(&self.0.id()).finish()
    }
}

impl std::ops::Deref for AppRef {
    type Target = dyn App + 'static;
    fn deref(&self) -> &(dyn App + 'static) {
        self.0
    }
}

/// The full catalog: every app that could ever run on this system, whether
/// installed or not. Add a new app here to register it.
pub const ALL: &[AppRef] = &[
    AppRef(&program_manager::ProgramManagerApp),
    AppRef(&notes::NotesApp),
    AppRef(&file_explorer::FileExplorerApp),
    AppRef(&contacts::ContactsApp),
    AppRef(&contacts_source::ContactsSourceApp),
    AppRef(&contacts_sort::ContactsSortApp),
    AppRef(&contacts_filter::ContactsFilterApp),
    AppRef(&workbench::WorkbenchApp),
    AppRef(&settings_manager::SettingsManagerApp),
    AppRef(&app_store::AppStoreApp),
    AppRef(&calculator::CalculatorApp),
];

#[derive(Clone, Copy, PartialEq)]
pub struct AppInstance {
    pub window_id: WindowId,
    pub workspace_id: WorkspaceId,
    pub pid: Pid,
    pub kind: AppRef,
}

/// Which apps are currently installed. The catalog ([`ALL`]) is every app
/// that could ever exist; this is the subset actually present on the
/// system — what the App Store shows as "Installed" and the desktop's
/// launcher is willing to run.
#[derive(Clone, Copy)]
pub struct InstalledApps {
    installed: RwSignal<HashSet<AppRef>>,
}

impl InstalledApps {
    pub fn new() -> Self {
        let installed = ALL.iter().copied().filter(|a| a.installed_by_default()).collect();
        Self {
            installed: RwSignal::new(installed),
        }
    }

    pub fn is_installed(&self, kind: AppRef) -> bool {
        self.installed.get().contains(&kind)
    }

    pub fn install(&self, kind: AppRef) {
        self.installed.update(|s| {
            s.insert(kind);
        });
    }

    pub fn uninstall(&self, kind: AppRef) {
        self.installed.update(|s| {
            s.remove(&kind);
        });
    }

    /// Reactive read access to the installed set.
    pub fn all(&self) -> RwSignal<HashSet<AppRef>> {
        self.installed
    }
}

impl Default for InstalledApps {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry of running app instances, layered on top of the OS-level
/// process table and per-workspace window managers.
///
/// Holds its dependencies by value rather than fetching them with
/// `expect_context` on demand: `Apps` methods are called from inside event
/// handlers, which run outside the reactive scope that makes context
/// lookups work, so the handles have to be captured up front instead.
#[derive(Clone, Copy)]
pub struct Apps {
    instances: RwSignal<Vec<AppInstance>>,
    workspaces: WorkspaceManager,
    installed: InstalledApps,
}

impl Apps {
    pub fn new(workspaces: WorkspaceManager, installed: InstalledApps) -> Self {
        Self {
            instances: RwSignal::new(Vec::new()),
            workspaces,
            installed,
        }
    }

    pub fn instances(&self) -> RwSignal<Vec<AppInstance>> {
        self.instances
    }

    pub fn installed(&self) -> InstalledApps {
        self.installed
    }

    /// Launches an app into the active workspace, spawning its process and
    /// opening its window there. If the app is a singleton and already
    /// running in that workspace, focuses it instead. No-ops for an app
    /// that isn't installed.
    pub fn launch(&self, kind: AppRef) {
        if !self.installed.is_installed(kind) {
            return;
        }

        let workspace = self.workspaces.active_workspace();

        if kind.is_singleton() {
            if let Some(existing) = self
                .instances
                .get_untracked()
                .iter()
                .find(|i| i.kind == kind && i.workspace_id == workspace.id)
            {
                workspace.focus(existing.window_id);
                return;
            }
        }

        let pid = kernel::spawn_process(kind.title(), kind.icon(), kind.id(), kind.role());
        let window_id = workspace.open_window(
            pid,
            kind.title(),
            kind.icon(),
            kind.preferred_size(),
            kind.size_constraints(),
        );

        self.instances.update(|i| {
            i.push(AppInstance {
                window_id,
                workspace_id: workspace.id,
                pid,
                kind,
            })
        });
    }

    /// The most recently launched running instance of `kind`, if any —
    /// used right after [`Apps::launch`] to find the pid/window of what
    /// was just opened (`launch` itself returns nothing, matching every
    /// other kernel-style wrapper in this codebase).
    pub fn last_instance_of(&self, kind: AppRef) -> Option<AppInstance> {
        self.instances.get_untracked().into_iter().rev().find(|i| i.kind == kind)
    }

    /// Spawns a headless instance of `kind` (no window, not tracked in
    /// [`Apps::instances`]) — used for the source and manipulator
    /// processes that back a pipeline, which the user builds through
    /// [`crate::app::pipeline`] rather than launching directly. Still
    /// visible in Program Manager, since it's a real kernel process.
    /// No-ops for an app that isn't installed.
    pub fn spawn_headless(&self, kind: AppRef) -> Option<Pid> {
        if !self.installed.is_installed(kind) {
            return None;
        }
        Some(kernel::spawn_process(kind.title(), kind.icon(), kind.id(), kind.role()))
    }

    /// Closes a running app instance, tearing down its window and process.
    pub fn close(&self, workspace_id: WorkspaceId, window_id: WindowId) {
        let Some(instance) = self
            .instances
            .get_untracked()
            .iter()
            .find(|i| i.workspace_id == workspace_id && i.window_id == window_id)
            .cloned()
        else {
            return;
        };

        if let Some(workspace) = self.workspaces.workspace(workspace_id) {
            workspace.close_window(window_id);
        }
        kernel::kill_process(instance.pid);
        self.instances.update(|i| {
            i.retain(|inst| !(inst.workspace_id == workspace_id && inst.window_id == window_id))
        });
    }

    /// Closes whichever app instance owns the given process, wherever it's
    /// running. Used by Program Manager, which only knows about pids.
    pub fn close_by_pid(&self, pid: Pid) {
        let Some(instance) = self
            .instances
            .get_untracked()
            .iter()
            .find(|i| i.pid == pid)
            .cloned()
        else {
            return;
        };
        self.close(instance.workspace_id, instance.window_id);
    }

    /// Installs an app from the catalog, making it launchable.
    pub fn install(&self, kind: AppRef) {
        self.installed.install(kind);
    }

    /// Uninstalls an app: closes every running instance of it, everywhere,
    /// then removes it from the installed set. If `remove_data` is set,
    /// also wipes its stored settings — the "also remove personalized
    /// data" choice the App Store asks about before uninstalling.
    pub fn uninstall(&self, kind: AppRef, remove_data: bool) {
        let running: Vec<(WorkspaceId, WindowId)> = self
            .instances
            .get_untracked()
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| (i.workspace_id, i.window_id))
            .collect();
        for (workspace_id, window_id) in running {
            self.close(workspace_id, window_id);
        }

        self.installed.uninstall(kind);

        if remove_data {
            kernel::clear_settings(Owner::App(kind.id()));
        }
    }
}

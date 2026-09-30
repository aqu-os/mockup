//! The kernel: the one place that owns process, window-identity, settings
//! and file-system state. Everything outside this module — window
//! managers, apps, UI — reaches it exclusively through [`syscall`], which
//! takes a [`Syscall`] describing the requested operation and returns a
//! [`SyscallResult`]. The enum *is* the kernel's API surface: if an
//! operation isn't a variant here, nothing outside the kernel can do it.
//!
//! [`syscall`] itself is deliberately awkward to call directly (match on
//! the result to get your value back out) — real syscalls are just a
//! number and a blob of arguments too. The `pub fn`s below it are thin,
//! ergonomic wrappers, the same way libc wraps `syscall(2)` in named
//! functions — everything outside the kernel should use those instead of
//! calling [`syscall`] itself.
//!
//! Window managers ([`crate::os::managers`], [`crate::os::window_manager`])
//! and workspaces ([`crate::os::workspace`]) are explicitly *not* part of
//! the kernel: they're policy (how a window's geometry is computed) built
//! on top of the identity/focus primitives the kernel provides, not state
//! the kernel itself needs to own.

mod data;
mod fs;
mod process;
mod settings;
mod window;

pub use data::{BlockId, DataBlock, Pipeline, PipelineId};
pub use fs::{DirNode, EntryMeta, FileKind, FileNode};
pub use process::{AppRole, Pid, Process};
pub use settings::{os_category, FieldKind, Owner, SettingField, SettingValue};
pub use window::{ResizeAxis, SizeConstraints, WindowId, WindowMeta, WorkspaceId};

use data::DataState;
use process::ProcessState;
use settings::SettingsState;
use window::WindowState;

/// Every operation the kernel supports. Constructed by callers, consumed
/// by [`syscall`] — this enum is the entire kernel API surface.
pub enum Syscall {
    SpawnProcess { name: String, icon: &'static str, app_id: &'static str, role: Option<AppRole> },
    KillProcess { pid: Pid },
    ListProcesses,
    OpenWindow { owner: Pid, workspace_id: WorkspaceId, title: String, icon: &'static str, constraints: SizeConstraints },
    CloseWindow { id: WindowId },
    FocusWindow { id: WindowId },
    FocusedWindow { workspace_id: WorkspaceId },
    WindowMeta { id: WindowId },
    WindowStackPosition { id: WindowId },
    GetSetting { owner: Owner, key: &'static str },
    SetSetting { owner: Owner, key: &'static str, value: SettingValue },
    ClearSettings { owner: Owner },
    ReadRootDir,
    /// Creates a data block, rejected unless `owner` is a running `Source`
    /// process — the OS-enforced boundary from root `CLAUDE.md`: only
    /// sources put new data into memory.
    CreateBlock { owner: Pid, schema: serde_json::Value, data: serde_json::Value },
    /// Overwrites a block's data, rejected unless `owner` is the block's
    /// existing owner — manipulators/sinks reading the same block cannot
    /// use this to mutate it out from under its source.
    UpdateBlock { block_id: BlockId, owner: Pid, data: serde_json::Value },
    ReadBlock { block_id: BlockId },
    BlockOwnedBy { owner: Pid },
    /// Starts a pipeline rooted at `source`, rejected unless `source` is a
    /// running `Source` process.
    CreatePipeline { source: Pid },
    /// Replaces a pipeline's manipulator chain wholesale (add/remove/
    /// reorder in one call), rejected unless every pid in `manipulators`
    /// is a running `Manipulator` process.
    SetManipulators { pipeline_id: PipelineId, manipulators: Vec<Pid> },
    /// Attaches a sink to a pipeline's output, rejected unless `sink` is a
    /// running `Sink` process.
    AttachSink { pipeline_id: PipelineId, sink: Pid },
    DetachSink { pipeline_id: PipelineId, sink: Pid },
    ListPipelines,
    ReadPipeline { pipeline_id: PipelineId },
    DeletePipeline { pipeline_id: PipelineId },
}

/// The result of a [`Syscall`]. Which variant comes back is determined by
/// which variant went in; the ergonomic wrapper functions below are what
/// unwrap the one they expect so callers never see this enum directly.
pub enum SyscallResult {
    Pid(Pid),
    Processes(Vec<Process>),
    WindowId(WindowId),
    WindowMeta(Option<WindowMeta>),
    FocusedWindow(Option<WindowId>),
    StackPosition(i32),
    Setting(Option<SettingValue>),
    Dir(DirNode),
    Block(Option<BlockId>),
    BlockData(Option<DataBlock>),
    CreatedPipeline(Option<PipelineId>),
    Pipelines(Vec<Pipeline>),
    PipelineMeta(Option<Pipeline>),
    Unit,
}

/// The kernel's global state. A copy-handle struct like every other store
/// in this codebase, just not context-provided: real syscalls don't
/// require "looking up a context" to reach the kernel, so this lives in a
/// `thread_local!` singleton instead, reachable from anywhere.
#[derive(Clone, Copy)]
struct Kernel {
    process: ProcessState,
    window: WindowState,
    settings: SettingsState,
    data: DataState,
}

thread_local! {
    static KERNEL: Kernel = Kernel {
        process: ProcessState::new(),
        window: WindowState::new(),
        settings: SettingsState::new(),
        data: DataState::new(),
    };
}

/// Forces the kernel's lazily-initialized state into existence right now,
/// under whichever reactive owner is currently active. Signals created
/// under a transient owner (say, some component's `Memo`) get disposed the
/// moment that owner tears down, which would silently brick every syscall
/// made afterwards — so this must be called once, up front, from the
/// app's permanent root scope, before anything else can touch the kernel
/// implicitly by triggering `KERNEL`'s lazy init from a shorter-lived one.
pub fn init() {
    KERNEL.with(|_| {});
}

/// The kernel's single entry point. Everything above this line in the
/// module is the only code allowed to touch `KERNEL`'s fields directly.
pub fn syscall(call: Syscall) -> SyscallResult {
    KERNEL.with(|k| match call {
        Syscall::SpawnProcess { name, icon, app_id, role } => {
            SyscallResult::Pid(k.process.spawn(name, icon, app_id, role))
        }
        Syscall::KillProcess { pid } => {
            k.process.kill(pid);
            SyscallResult::Unit
        }
        Syscall::ListProcesses => SyscallResult::Processes(k.process.all()),
        Syscall::OpenWindow { owner, workspace_id, title, icon, constraints } => {
            SyscallResult::WindowId(k.window.open(workspace_id, owner, title, icon, constraints))
        }
        Syscall::CloseWindow { id } => {
            k.window.close(id);
            SyscallResult::Unit
        }
        Syscall::FocusWindow { id } => {
            k.window.focus(id);
            SyscallResult::Unit
        }
        Syscall::FocusedWindow { workspace_id } => SyscallResult::FocusedWindow(k.window.focused(workspace_id)),
        Syscall::WindowMeta { id } => SyscallResult::WindowMeta(k.window.meta(id)),
        Syscall::WindowStackPosition { id } => SyscallResult::StackPosition(k.window.stack_position(id)),
        Syscall::GetSetting { owner, key } => SyscallResult::Setting(k.settings.get(owner, key)),
        Syscall::SetSetting { owner, key, value } => {
            k.settings.set(owner, key, value);
            SyscallResult::Unit
        }
        Syscall::ClearSettings { owner } => {
            k.settings.clear_owner(owner);
            SyscallResult::Unit
        }
        Syscall::ReadRootDir => SyscallResult::Dir(fs::root()),
        Syscall::CreateBlock { owner, schema, data } => {
            if k.process.role(owner) != Some(AppRole::Source) {
                SyscallResult::Block(None)
            } else {
                SyscallResult::Block(Some(k.data.create_block(owner, schema, data)))
            }
        }
        Syscall::UpdateBlock { block_id, owner, data } => {
            k.data.update_block(block_id, owner, data);
            SyscallResult::Unit
        }
        Syscall::ReadBlock { block_id } => SyscallResult::BlockData(k.data.read_block(block_id)),
        Syscall::BlockOwnedBy { owner } => SyscallResult::Block(k.data.block_owned_by(owner)),
        Syscall::CreatePipeline { source } => {
            if k.process.role(source) != Some(AppRole::Source) {
                SyscallResult::CreatedPipeline(None)
            } else {
                SyscallResult::CreatedPipeline(Some(k.data.create_pipeline(source)))
            }
        }
        Syscall::SetManipulators { pipeline_id, manipulators } => {
            if manipulators.iter().all(|&pid| k.process.role(pid) == Some(AppRole::Manipulator)) {
                k.data.set_manipulators(pipeline_id, manipulators);
            }
            SyscallResult::Unit
        }
        Syscall::AttachSink { pipeline_id, sink } => {
            if k.process.role(sink) == Some(AppRole::Sink) {
                k.data.attach_sink(pipeline_id, sink);
            }
            SyscallResult::Unit
        }
        Syscall::DetachSink { pipeline_id, sink } => {
            k.data.detach_sink(pipeline_id, sink);
            SyscallResult::Unit
        }
        Syscall::ListPipelines => SyscallResult::Pipelines(k.data.list_pipelines()),
        Syscall::ReadPipeline { pipeline_id } => SyscallResult::PipelineMeta(k.data.read_pipeline(pipeline_id)),
        Syscall::DeletePipeline { pipeline_id } => {
            k.data.delete_pipeline(pipeline_id);
            SyscallResult::Unit
        }
    })
}

pub fn spawn_process(
    name: impl Into<String>,
    icon: &'static str,
    app_id: &'static str,
    role: Option<AppRole>,
) -> Pid {
    match syscall(Syscall::SpawnProcess { name: name.into(), icon, app_id, role }) {
        SyscallResult::Pid(pid) => pid,
        _ => unreachable!(),
    }
}

pub fn kill_process(pid: Pid) {
    syscall(Syscall::KillProcess { pid });
}

pub fn list_processes() -> Vec<Process> {
    match syscall(Syscall::ListProcesses) {
        SyscallResult::Processes(procs) => procs,
        _ => unreachable!(),
    }
}

pub fn open_window(
    owner: Pid,
    workspace_id: WorkspaceId,
    title: impl Into<String>,
    icon: &'static str,
    constraints: SizeConstraints,
) -> WindowId {
    match syscall(Syscall::OpenWindow { owner, workspace_id, title: title.into(), icon, constraints }) {
        SyscallResult::WindowId(id) => id,
        _ => unreachable!(),
    }
}

pub fn close_window(id: WindowId) {
    syscall(Syscall::CloseWindow { id });
}

pub fn focus_window(id: WindowId) {
    syscall(Syscall::FocusWindow { id });
}

pub fn focused_window(workspace_id: WorkspaceId) -> Option<WindowId> {
    match syscall(Syscall::FocusedWindow { workspace_id }) {
        SyscallResult::FocusedWindow(id) => id,
        _ => unreachable!(),
    }
}

pub fn window_meta(id: WindowId) -> Option<WindowMeta> {
    match syscall(Syscall::WindowMeta { id }) {
        SyscallResult::WindowMeta(meta) => meta,
        _ => unreachable!(),
    }
}

pub fn window_stack_position(id: WindowId) -> i32 {
    match syscall(Syscall::WindowStackPosition { id }) {
        SyscallResult::StackPosition(pos) => pos,
        _ => unreachable!(),
    }
}

fn get_setting(owner: Owner, key: &'static str) -> Option<SettingValue> {
    match syscall(Syscall::GetSetting { owner, key }) {
        SyscallResult::Setting(value) => value,
        _ => unreachable!(),
    }
}

pub fn get_setting_bool(owner: Owner, key: &'static str, default: bool) -> bool {
    match get_setting(owner, key) {
        Some(SettingValue::Bool(b)) => b,
        _ => default,
    }
}

pub fn get_setting_number(owner: Owner, key: &'static str, default: f64) -> f64 {
    match get_setting(owner, key) {
        Some(SettingValue::Number(n)) => n,
        _ => default,
    }
}

pub fn get_setting_choice(owner: Owner, key: &'static str, default: usize) -> usize {
    match get_setting(owner, key) {
        Some(SettingValue::Choice(c)) => c,
        _ => default,
    }
}

pub fn get_setting_text(owner: Owner, key: &'static str, default: &str) -> String {
    match get_setting(owner, key) {
        Some(SettingValue::Text(t)) => t,
        _ => default.to_string(),
    }
}

/// Reads a stored setting for `field`, falling back to its declared
/// default — the generic form behind the per-kind `get_setting_*` helpers
/// above, useful when a field's `FieldKind` isn't known ahead of time
/// (e.g. building a manipulator's parameter map generically from its
/// [`SettingField`] list — see `app::pipeline`).
pub fn get_setting_value(owner: Owner, field: &SettingField) -> SettingValue {
    match field.kind {
        FieldKind::Toggle => {
            let default = matches!(field.default, SettingValue::Bool(true));
            SettingValue::Bool(get_setting_bool(owner, field.key, default))
        }
        FieldKind::Number { min, .. } => {
            let default = match field.default {
                SettingValue::Number(n) => n,
                _ => min,
            };
            SettingValue::Number(get_setting_number(owner, field.key, default))
        }
        FieldKind::Choice(_) => {
            let default = match field.default {
                SettingValue::Choice(c) => c,
                _ => 0,
            };
            SettingValue::Choice(get_setting_choice(owner, field.key, default))
        }
        FieldKind::Text { .. } => {
            let default = match &field.default {
                SettingValue::Text(t) => t.as_str(),
                _ => "",
            };
            SettingValue::Text(get_setting_text(owner, field.key, default))
        }
    }
}

pub fn set_setting(owner: Owner, key: &'static str, value: SettingValue) {
    syscall(Syscall::SetSetting { owner, key, value });
}

pub fn clear_settings(owner: Owner) {
    syscall(Syscall::ClearSettings { owner });
}

pub fn read_root_dir() -> DirNode {
    match syscall(Syscall::ReadRootDir) {
        SyscallResult::Dir(dir) => dir,
        _ => unreachable!(),
    }
}

/// Creates a data block owned by `owner`. Returns `None` if `owner` isn't
/// a running `Source` process — see [`Syscall::CreateBlock`].
pub fn create_block(owner: Pid, schema: serde_json::Value, data: serde_json::Value) -> Option<BlockId> {
    match syscall(Syscall::CreateBlock { owner, schema, data }) {
        SyscallResult::Block(id) => id,
        _ => unreachable!(),
    }
}

/// Overwrites a block's data. No-ops if `owner` isn't the block's existing
/// owner — see [`Syscall::UpdateBlock`].
pub fn update_block(block_id: BlockId, owner: Pid, data: serde_json::Value) {
    syscall(Syscall::UpdateBlock { block_id, owner, data });
}

pub fn read_block(block_id: BlockId) -> Option<DataBlock> {
    match syscall(Syscall::ReadBlock { block_id }) {
        SyscallResult::BlockData(block) => block,
        _ => unreachable!(),
    }
}

pub fn block_owned_by(owner: Pid) -> Option<BlockId> {
    match syscall(Syscall::BlockOwnedBy { owner }) {
        SyscallResult::Block(id) => id,
        _ => unreachable!(),
    }
}

/// Starts a pipeline rooted at `source`. Returns `None` if `source` isn't
/// a running `Source` process — see [`Syscall::CreatePipeline`].
pub fn create_pipeline(source: Pid) -> Option<PipelineId> {
    match syscall(Syscall::CreatePipeline { source }) {
        SyscallResult::CreatedPipeline(id) => id,
        _ => unreachable!(),
    }
}

/// Replaces a pipeline's manipulator chain wholesale. No-ops (chain left
/// unchanged) unless every pid is a running `Manipulator` process.
pub fn set_manipulators(pipeline_id: PipelineId, manipulators: Vec<Pid>) {
    syscall(Syscall::SetManipulators { pipeline_id, manipulators });
}

/// No-ops unless `sink` is a running `Sink` process.
pub fn attach_sink(pipeline_id: PipelineId, sink: Pid) {
    syscall(Syscall::AttachSink { pipeline_id, sink });
}

pub fn detach_sink(pipeline_id: PipelineId, sink: Pid) {
    syscall(Syscall::DetachSink { pipeline_id, sink });
}

pub fn list_pipelines() -> Vec<Pipeline> {
    match syscall(Syscall::ListPipelines) {
        SyscallResult::Pipelines(pipelines) => pipelines,
        _ => unreachable!(),
    }
}

pub fn read_pipeline(pipeline_id: PipelineId) -> Option<Pipeline> {
    match syscall(Syscall::ReadPipeline { pipeline_id }) {
        SyscallResult::PipelineMeta(pipeline) => pipeline,
        _ => unreachable!(),
    }
}

pub fn delete_pipeline(pipeline_id: PipelineId) {
    syscall(Syscall::DeletePipeline { pipeline_id });
}

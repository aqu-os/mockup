//! Kernel-private process state: every running "program" in the mockup,
//! independent of whether it has a window — the same way a real OS tracks
//! processes separate from any GUI surface they own. Reached only through
//! [`super::syscall`].

use leptos::prelude::*;

pub type Pid = u32;

/// The three OS-enforced app roles from the source/manipulator/sink model
/// (see root `CLAUDE.md`), plus the implicit "none" for apps that don't
/// participate in a data pipeline at all (Settings, Program Manager, ...).
/// Stored per-process so the kernel can gate data-block and pipeline
/// syscalls against it — see `super::data`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppRole {
    Source,
    Manipulator,
    Sink,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: Pid,
    pub name: String,
    pub icon: &'static str,
    /// The owning app's stable id (see `Owner::App`), used to resolve a
    /// process back to its `app::AppRef` from outside the kernel (the
    /// kernel itself doesn't know about `app::AppRef`).
    pub app_id: &'static str,
    pub role: Option<AppRole>,
}

/// A handle to the kernel's process table, held by the global [`super::Kernel`].
#[derive(Clone, Copy)]
pub(super) struct ProcessState {
    processes: RwSignal<Vec<Process>>,
    next_pid: RwSignal<Pid>,
}

impl ProcessState {
    pub(super) fn new() -> Self {
        Self {
            processes: RwSignal::new(Vec::new()),
            next_pid: RwSignal::new(1),
        }
    }

    pub(super) fn spawn(
        &self,
        name: String,
        icon: &'static str,
        app_id: &'static str,
        role: Option<AppRole>,
    ) -> Pid {
        let pid = self.next_pid.get_untracked();
        self.next_pid.set(pid + 1);
        self.processes.update(|procs| procs.push(Process { pid, name, icon, app_id, role }));
        pid
    }

    pub(super) fn kill(&self, pid: Pid) {
        self.processes.update(|procs| procs.retain(|p| p.pid != pid));
    }

    pub(super) fn all(&self) -> Vec<Process> {
        self.processes.get()
    }

    pub(super) fn role(&self, pid: Pid) -> Option<AppRole> {
        self.processes.get_untracked().iter().find(|p| p.pid == pid).and_then(|p| p.role)
    }
}

//! Kernel-private data-block and pipeline state: the in-memory JSON
//! records a `Source` app owns, and the pipelines that chain a source
//! through zero or more manipulators into one or more sinks — the
//! mockup's stand-in for the "memory blocks" and OS-enforced app roles
//! described in the root `CLAUDE.md`. Real typed/binary memory is
//! deferred; everything here is plain JSON, but the role checks in
//! [`super::syscall`] that gate access to it are real: only the block's
//! owning `Source` process may create or update it, and only `Sink`
//! processes may be attached to a pipeline as its outputs. Reached only
//! through [`super::syscall`].

use leptos::prelude::*;

use super::process::Pid;

pub type BlockId = u32;
pub type PipelineId = u32;

#[derive(Clone, Debug)]
pub struct DataBlock {
    pub id: BlockId,
    pub owner: Pid,
    pub schema: serde_json::Value,
    pub data: serde_json::Value,
}

/// A source, chained through an ordered list of manipulators, feeding one
/// or more sinks. Order matters: manipulators are applied in sequence, so
/// e.g. filtering before sorting can give a different result than sorting
/// before filtering.
#[derive(Clone, Debug)]
pub struct Pipeline {
    pub id: PipelineId,
    pub source: Pid,
    pub manipulators: Vec<Pid>,
    pub sinks: Vec<Pid>,
}

#[derive(Clone, Copy)]
pub(super) struct DataState {
    blocks: RwSignal<Vec<DataBlock>>,
    next_block_id: RwSignal<BlockId>,
    pipelines: RwSignal<Vec<Pipeline>>,
    next_pipeline_id: RwSignal<PipelineId>,
}

impl DataState {
    pub(super) fn new() -> Self {
        Self {
            blocks: RwSignal::new(Vec::new()),
            next_block_id: RwSignal::new(1),
            pipelines: RwSignal::new(Vec::new()),
            next_pipeline_id: RwSignal::new(1),
        }
    }

    pub(super) fn create_block(&self, owner: Pid, schema: serde_json::Value, data: serde_json::Value) -> BlockId {
        let id = self.next_block_id.get_untracked();
        self.next_block_id.set(id + 1);
        self.blocks.update(|b| b.push(DataBlock { id, owner, schema, data }));
        id
    }

    /// Applies only if `owner` matches the block's existing owner — the
    /// enforcement point for "manipulators/sinks can't write a source's
    /// data". Returns whether the update was applied.
    pub(super) fn update_block(&self, block_id: BlockId, owner: Pid, data: serde_json::Value) -> bool {
        let mut applied = false;
        self.blocks.update(|blocks| {
            if let Some(block) = blocks.iter_mut().find(|b| b.id == block_id) {
                if block.owner == owner {
                    block.data = data;
                    applied = true;
                }
            }
        });
        applied
    }

    pub(super) fn read_block(&self, block_id: BlockId) -> Option<DataBlock> {
        self.blocks.get().into_iter().find(|b| b.id == block_id)
    }

    pub(super) fn block_owned_by(&self, owner: Pid) -> Option<BlockId> {
        self.blocks.get_untracked().iter().find(|b| b.owner == owner).map(|b| b.id)
    }

    pub(super) fn create_pipeline(&self, source: Pid) -> PipelineId {
        let id = self.next_pipeline_id.get_untracked();
        self.next_pipeline_id.set(id + 1);
        self.pipelines.update(|p| {
            p.push(Pipeline { id, source, manipulators: Vec::new(), sinks: Vec::new() })
        });
        id
    }

    pub(super) fn set_manipulators(&self, pipeline_id: PipelineId, manipulators: Vec<Pid>) {
        self.pipelines.update(|pipelines| {
            if let Some(p) = pipelines.iter_mut().find(|p| p.id == pipeline_id) {
                p.manipulators = manipulators;
            }
        });
    }

    pub(super) fn attach_sink(&self, pipeline_id: PipelineId, sink: Pid) {
        self.pipelines.update(|pipelines| {
            if let Some(p) = pipelines.iter_mut().find(|p| p.id == pipeline_id) {
                if !p.sinks.contains(&sink) {
                    p.sinks.push(sink);
                }
            }
        });
    }

    pub(super) fn detach_sink(&self, pipeline_id: PipelineId, sink: Pid) {
        self.pipelines.update(|pipelines| {
            if let Some(p) = pipelines.iter_mut().find(|p| p.id == pipeline_id) {
                p.sinks.retain(|&s| s != sink);
            }
        });
    }

    pub(super) fn list_pipelines(&self) -> Vec<Pipeline> {
        self.pipelines.get()
    }

    pub(super) fn read_pipeline(&self, pipeline_id: PipelineId) -> Option<Pipeline> {
        self.pipelines.get().into_iter().find(|p| p.id == pipeline_id)
    }

    pub(super) fn delete_pipeline(&self, pipeline_id: PipelineId) {
        self.pipelines.update(|p| p.retain(|p| p.id != pipeline_id));
    }
}

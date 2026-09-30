//! Policy layer for wiring source/manipulator/sink apps into pipelines.
//! Lives here, in `app::`, rather than `os::` — the kernel tracks
//! pipeline *topology* (which pids are chained together, see
//! `os::kernel::Pipeline`) without knowing anything about `app::AppRef`;
//! this module is the glue one layer up that resolves a process id back
//! to the concrete app behind it and actually runs the data-transform
//! chain. Mirrors how `os::workspace` is policy layered on top of the
//! kernel's window-identity primitives.

use std::collections::HashMap;

use leptos::prelude::*;

use super::settings_manager::render_fields;
use super::{Action, AppRef, Apps, ALL};
use crate::os::kernel::{self as kernel, AppRole, BlockId, Owner, Pid, Pipeline, PipelineId, SettingValue};

/// Resolves a running process back to the concrete app behind it, via the
/// stable app id the kernel stores per-process. The kernel itself stays
/// ignorant of `AppRef` — this is the only place that bridges the two.
pub fn resolve(pid: Pid) -> Option<AppRef> {
    let app_id = kernel::list_processes().into_iter().find(|p| p.pid == pid)?.app_id;
    ALL.iter().copied().find(|a| a.id() == app_id)
}

/// Every installed app with the given role — for source/manipulator
/// pickers in the Workbench and inline pipeline builder.
pub fn installed_with_role(apps: Apps, role: AppRole) -> Vec<AppRef> {
    let installed = apps.installed().all().get();
    ALL.iter().copied().filter(|a| a.role() == Some(role) && installed.contains(a)).collect()
}

/// Finds an already-running instance of `app`, if any. Sources in this
/// mockup are singletons, so building a second pipeline on the same
/// source reuses its existing process and block rather than spawning a
/// duplicate.
fn running_instance(app: AppRef) -> Option<Pid> {
    kernel::list_processes().into_iter().find(|p| p.app_id == app.id()).map(|p| p.pid)
}

/// Ensures `source`'s process and owned data block exist, reusing a
/// running instance if there is one.
fn ensure_source(apps: Apps, source: AppRef) -> Option<Pid> {
    let pid = match running_instance(source) {
        Some(pid) => pid,
        None => apps.spawn_headless(source)?,
    };
    if kernel::block_owned_by(pid).is_none() {
        kernel::create_block(pid, source.schema(), source.seed());
    }
    Some(pid)
}

/// Starts a new pipeline rooted at `source`.
pub fn create_pipeline(apps: Apps, source: AppRef) -> Option<PipelineId> {
    let source_pid = ensure_source(apps, source)?;
    kernel::create_pipeline(source_pid)
}

/// Appends a fresh instance of `manipulator` to the end of a pipeline's
/// chain.
pub fn add_manipulator(apps: Apps, pipeline_id: PipelineId, manipulator: AppRef) -> Option<Pid> {
    let pid = apps.spawn_headless(manipulator)?;
    let mut pipeline = kernel::read_pipeline(pipeline_id)?;
    pipeline.manipulators.push(pid);
    kernel::set_manipulators(pipeline_id, pipeline.manipulators);
    Some(pid)
}

/// Removes a manipulator instance from its pipeline and kills its process
/// — it isn't shared with any other pipeline, so nothing else depends on
/// it staying alive.
pub fn remove_manipulator(pipeline_id: PipelineId, manipulator_pid: Pid) {
    let Some(mut pipeline) = kernel::read_pipeline(pipeline_id) else {
        return;
    };
    pipeline.manipulators.retain(|&p| p != manipulator_pid);
    kernel::set_manipulators(pipeline_id, pipeline.manipulators);
    kernel::kill_process(manipulator_pid);
}

/// Moves a manipulator one step earlier (`delta = -1`) or later
/// (`delta = 1`) in the chain. Order matters — filtering before sorting
/// can give a different result than sorting before filtering.
pub fn move_manipulator(pipeline_id: PipelineId, manipulator_pid: Pid, delta: i32) {
    let Some(mut pipeline) = kernel::read_pipeline(pipeline_id) else {
        return;
    };
    let Some(idx) = pipeline.manipulators.iter().position(|&p| p == manipulator_pid) else {
        return;
    };
    let new_idx = (idx as i32 + delta).clamp(0, pipeline.manipulators.len() as i32 - 1) as usize;
    if new_idx != idx {
        pipeline.manipulators.swap(idx, new_idx);
        kernel::set_manipulators(pipeline_id, pipeline.manipulators);
    }
}

pub fn attach_sink(pipeline_id: PipelineId, sink_pid: Pid) {
    kernel::attach_sink(pipeline_id, sink_pid);
}

pub fn detach_sink(pipeline_id: PipelineId, sink_pid: Pid) {
    kernel::detach_sink(pipeline_id, sink_pid);
}

/// Tears down a pipeline entirely: kills its manipulator processes (which
/// belong to it alone) and drops the pipeline record. Leaves the source
/// running, since it's a singleton other pipelines could still point at.
pub fn delete_pipeline(pipeline_id: PipelineId) {
    if let Some(pipeline) = kernel::read_pipeline(pipeline_id) {
        for pid in pipeline.manipulators {
            kernel::kill_process(pid);
        }
    }
    kernel::delete_pipeline(pipeline_id);
}

/// Current stored parameter values for one manipulator *instance* (keyed
/// by its pid via `Owner::Process` — see `os::kernel::settings`), falling
/// back to each field's declared default.
pub fn manipulator_params(app: AppRef, pid: Pid) -> HashMap<&'static str, SettingValue> {
    app.param_schema()
        .iter()
        .map(|field| (field.key, kernel::get_setting_value(Owner::Process(pid), field)))
        .collect()
}

/// Computes a pipeline's current output: reads the source's block, then
/// folds each manipulator's `apply()` over it in order.
pub fn compute(pipeline_id: PipelineId) -> serde_json::Value {
    let Some(pipeline) = kernel::read_pipeline(pipeline_id) else {
        return serde_json::Value::Null;
    };
    let Some(block_id) = kernel::block_owned_by(pipeline.source) else {
        return serde_json::Value::Null;
    };
    let Some(block) = kernel::read_block(block_id) else {
        return serde_json::Value::Null;
    };
    pipeline.manipulators.into_iter().fold(block.data, |data, pid| match resolve(pid) {
        Some(app) => {
            let params = manipulator_params(app, pid);
            app.apply(data, &params)
        }
        None => data,
    })
}

/// Every pipeline with `sink_pid` attached as one of its outputs. A sink
/// window uses this on first render to find whether it already has a
/// pipeline (e.g. reopening a singleton sink) or needs to show the
/// builder.
pub fn pipelines_for_sink(sink_pid: Pid) -> Vec<Pipeline> {
    kernel::list_pipelines().into_iter().filter(|p| p.sinks.contains(&sink_pid)).collect()
}

/// A compact, read-only summary of a pipeline's chain — icon + title per
/// step, no editing controls. Used in a sink's header once it has a
/// pipeline attached; see [`PipelineChain`] for the editable version.
#[component]
pub fn PipelineSummary(pipeline_id: PipelineId) -> impl IntoView {
    move || {
        let Some(pipeline) = kernel::read_pipeline(pipeline_id) else {
            return view! { <div class="pipeline-summary">"No pipeline."</div> }.into_any();
        };
        let source = resolve(pipeline.source);
        let steps: Vec<AppRef> = pipeline.manipulators.iter().filter_map(|&pid| resolve(pid)).collect();
        view! {
            <div class="pipeline-summary">
                <span class="pipeline-summary__chip">
                    <span class="pipeline-summary__icon">{source.map(|a| a.icon()).unwrap_or("?")}</span>
                    {source.map(|a| a.title()).unwrap_or("Unknown source")}
                </span>
                <For each=move || steps.clone() key=|a| a.id() let(step)>
                    <span class="pipeline-summary__arrow">"\u{2192}"</span>
                    <span class="pipeline-summary__chip">
                        <span class="pipeline-summary__icon">{step.icon()}</span>
                        {step.title()}
                    </span>
                </For>
            </div>
        }
            .into_any()
    }
}

/// One manipulator's position in a chain, for `<For>`'s item template
/// below — kept as a named struct rather than a `(usize, Pid)` tuple
/// since the `view!` macro's `key`/`let` sugar wants a single-identifier
/// pattern, not a tuple destructure.
#[derive(Clone, Copy)]
struct ManipulatorSlot {
    index: usize,
    pid: Pid,
}

/// A small chip strip showing a pipeline's chain (source → manipulators)
/// with controls to add/remove/reorder manipulators and edit each one's
/// parameters — the reusable core shared by the Workbench (one chip strip
/// per pipeline) and the inline builder embedded in a sink's own window.
#[component]
pub fn PipelineChain(apps: Apps, pipeline_id: PipelineId) -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let pipeline = move || {
        refresh.track();
        kernel::read_pipeline(pipeline_id)
    };
    let bump = move || refresh.update(|n| *n += 1);

    let available_manipulators = move || {
        let used: Vec<&'static str> = pipeline()
            .map(|p| p.manipulators.iter().filter_map(|&pid| resolve(pid)).map(|a| a.id()).collect())
            .unwrap_or_default();
        installed_with_role(apps, AppRole::Manipulator)
            .into_iter()
            .filter(move |a| !used.contains(&a.id()))
            .collect::<Vec<_>>()
    };

    view! {
        <div class="pipeline-chain">
            {move || {
                let Some(pipeline) = pipeline() else {
                    return view! { <div class="pipeline-chain__empty">"No pipeline."</div> }.into_any();
                };
                let source = resolve(pipeline.source);
                let manipulators = pipeline.manipulators.clone();
                let manipulator_count = manipulators.len();
                view! {
                    <div class="pipeline-chain__chips">
                        <div class="pipeline-chip pipeline-chip--source">
                            <span class="pipeline-chip__icon">
                                {source.map(|a| a.icon()).unwrap_or("?")}
                            </span>
                            <span>{source.map(|a| a.title()).unwrap_or("Unknown source")}</span>
                        </div>
                        <For
                            each=move || {
                                manipulators
                                    .iter()
                                    .enumerate()
                                    .map(|(index, &pid)| ManipulatorSlot { index, pid })
                                    .collect::<Vec<_>>()
                            }
                            key=|slot| slot.pid
                            let(slot)
                        >
                            <span class="pipeline-chain__arrow">"\u{2192}"</span>
                            <div class="pipeline-chip pipeline-chip--manipulator">
                                {move || match resolve(slot.pid) {
                                    Some(app) => {
                                        view! {
                                            <span class="pipeline-chip__icon">{app.icon()}</span>
                                            <span>{app.title()}</span>
                                            <div class="pipeline-chip__params">
                                                {render_fields(Owner::Process(slot.pid), app.param_schema())}
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    None => view! { <span>"?"</span> }.into_any(),
                                }}
                                <div class="pipeline-chip__actions">
                                    <button
                                        class="pipeline-chip__btn"
                                        disabled=slot.index == 0
                                        on:click=move |_| {
                                            move_manipulator(pipeline_id, slot.pid, -1);
                                            bump();
                                        }
                                    >
                                        "\u{2190}"
                                    </button>
                                    <button
                                        class="pipeline-chip__btn"
                                        disabled=slot.index + 1 == manipulator_count
                                        on:click=move |_| {
                                            move_manipulator(pipeline_id, slot.pid, 1);
                                            bump();
                                        }
                                    >
                                        "\u{2192}"
                                    </button>
                                    <button
                                        class="pipeline-chip__btn pipeline-chip__btn--remove"
                                        on:click=move |_| {
                                            remove_manipulator(pipeline_id, slot.pid);
                                            bump();
                                        }
                                    >
                                        "\u{2715}"
                                    </button>
                                </div>
                            </div>
                        </For>
                    </div>
                    <div class="pipeline-chain__add">
                        <For each=available_manipulators key=|a| a.id() let(manipulator)>
                            <button
                                class="pipeline-chain__add-btn"
                                on:click=move |_| {
                                    add_manipulator(apps, pipeline_id, manipulator);
                                    bump();
                                }
                            >
                                {format!("+ {}", manipulator.title())}
                            </button>
                        </For>
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}

/// A row of buttons, one per installed app with `role`, for picking a
/// source or sink app. Used for a source/new-pipeline picker (`role =
/// Source`) in the Workbench and a sink's inline builder, and for an
/// "attach a sink" picker (`role = Sink`) in the generic source inspector.
#[component]
pub fn AppPicker(apps: Apps, role: AppRole, on_pick: impl Fn(AppRef) + Send + Sync + 'static + Copy) -> impl IntoView {
    let choices = move || installed_with_role(apps, role);
    view! {
        <div class="app-picker">
            <For each=choices key=|a| a.id() let(choice)>
                <button class="app-picker__btn" on:click=move |_| on_pick(choice)>
                    <span class="app-picker__icon">{choice.icon()}</span>
                    <span>{choice.title()}</span>
                </button>
            </For>
        </div>
    }
}

/// The first pipeline (if any) already rooted at this source — used to
/// reuse a pipeline rather than always starting a fresh one.
pub fn pipeline_for_source(source_pid: Pid) -> Option<PipelineId> {
    kernel::list_pipelines().into_iter().find(|p| p.source == source_pid).map(|p| p.id)
}

/// Every pipeline rooted at this source — a source can in principle feed
/// more than one, e.g. via the Workbench's "New pipeline" picker. Used
/// when tearing a source down entirely (see `app_store.rs`'s uninstall).
pub fn pipelines_for_source(source_pid: Pid) -> Vec<Pipeline> {
    kernel::list_pipelines().into_iter().filter(|p| p.source == source_pid).collect()
}

/// One declared [`Action`], rendered as a button that expands into a
/// one-shot form for its params (ephemeral input state — unlike a
/// manipulator's persisted params, an action's arguments aren't saved
/// anywhere, just sent once). Used by the generic source inspector;
/// bespoke sinks (like Contacts) are free to build their own UI around
/// `invoke_action` instead of this generic form — see `contacts.rs`.
#[component]
pub fn ActionButton(source: AppRef, block_id: BlockId, owner: Pid, action: &'static Action) -> impl IntoView {
    let open = RwSignal::new(false);
    let values = RwSignal::new(HashMap::<&'static str, String>::new());

    let submit = move |_| {
        let raw = values.get_untracked();
        let params: HashMap<&'static str, SettingValue> = action
            .params
            .iter()
            .map(|field| {
                let text = raw.get(field.key).cloned().unwrap_or_default();
                let value = match field.kind {
                    crate::os::kernel::FieldKind::Number { .. } => {
                        SettingValue::Number(text.parse().unwrap_or(0.0))
                    }
                    _ => SettingValue::Text(text),
                };
                (field.key, value)
            })
            .collect();
        source.invoke_action(block_id, owner, action.id, &params);
        values.set(HashMap::new());
        open.set(false);
    };

    view! {
        <div class="action-button">
            <button class="action-button__toggle" on:click=move |_| open.update(|o| *o = !*o)>
                {action.label}
            </button>
            <Show when=move || open.get()>
                <div class="action-button__form">
                    <For each=move || action.params key=|f| f.key let(field)>
                        <input
                            class="action-button__input"
                            type="text"
                            placeholder=field.label
                            on:input=move |ev| {
                                let text = event_target_value(&ev);
                                values.update(|v| {
                                    v.insert(field.key, text);
                                });
                            }
                        />
                    </For>
                    <button class="action-button__submit" on:click=submit>
                        "Go"
                    </button>
                </div>
            </Show>
        </div>
    }
}

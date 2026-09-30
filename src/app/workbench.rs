//! Workbench: lists every pipeline in the system and lets the user
//! create, edit, and tear them down — the "workshop" view of the
//! source/manipulator/sink model, complementing the inline builder a sink
//! shows when opened directly with no pipeline attached yet (see
//! `contacts.rs`). A system app like Program Manager or Settings: it
//! doesn't itself participate in a pipeline (`role()` stays `None`), it
//! manages them.

use leptos::prelude::*;

use super::pipeline::{self, AppPicker, PipelineChain};
use super::{App, AppRef, Apps};
use crate::os::kernel::{self as kernel, AppRole, Pid, PipelineId, SizeConstraints};

pub struct WorkbenchApp;

impl App for WorkbenchApp {
    fn id(&self) -> &'static str {
        "workbench"
    }
    fn title(&self) -> &'static str {
        "Workbench"
    }
    fn description(&self) -> &'static str {
        "Build and edit data pipelines: source \u{2192} manipulators \u{2192} sinks."
    }
    fn icon(&self) -> &'static str {
        "\u{1F6E0}\u{FE0F}"
    }
    fn is_system(&self) -> bool {
        true
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (560.0, 400.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 420.0,
            min_height: 300.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <Workbench /> }.into_any()
    }
}

#[component]
fn Workbench() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let refresh = RwSignal::new(0u32);
    let bump = move || refresh.update(|n| *n += 1);
    let creating = RwSignal::new(false);

    let pipelines = move || {
        refresh.track();
        kernel::list_pipelines()
    };

    let create = move |source: AppRef| {
        pipeline::create_pipeline(apps, source);
        creating.set(false);
        bump();
    };

    view! {
        <div class="workbench">
            <div class="workbench__toolbar">
                <button class="workbench__new-btn" on:click=move |_| creating.update(|c| *c = !*c)>
                    "+ New pipeline"
                </button>
                <Show when=move || creating.get()>
                    <AppPicker apps=apps role=AppRole::Source on_pick=create />
                </Show>
            </div>
            <div class="workbench__list">
                <Show
                    when=move || !pipelines().is_empty()
                    fallback=|| view! { <div class="workbench__empty">"No pipelines yet."</div> }
                >
                    <For each=pipelines key=|p| p.id let(pipeline)>
                        {
                            let pipeline_id = pipeline.id;
                            let sinks = pipeline.sinks.clone();
                            view! {
                                <div class="workbench__row">
                                    <PipelineChain apps=apps pipeline_id=pipeline_id />
                                    <div class="workbench__sinks">
                                        <For each=move || sinks.clone() key=|pid| *pid let(sink_pid)>
                                            <span class="workbench__sink-chip">
                                                {move || pipeline::resolve(sink_pid).map(|a| a.title()).unwrap_or("?")}
                                                <button
                                                    class="workbench__sink-detach"
                                                    on:click=move |_| {
                                                        pipeline::detach_sink(pipeline_id, sink_pid);
                                                        bump();
                                                    }
                                                >
                                                    "\u{2715}"
                                                </button>
                                            </span>
                                        </For>
                                        <AttachSinkButton apps=apps pipeline_id=pipeline_id on_attach=bump />
                                    </div>
                                    <button
                                        class="workbench__delete-btn"
                                        on:click=move |_| {
                                            pipeline::delete_pipeline(pipeline_id);
                                            bump();
                                        }
                                    >
                                        "Delete pipeline"
                                    </button>
                                </div>
                            }
                        }
                    </For>
                </Show>
            </div>
        </div>
    }
}

#[component]
fn AttachSinkButton(apps: Apps, pipeline_id: PipelineId, on_attach: impl Fn() + Send + Sync + 'static + Copy) -> impl IntoView {
    let open = RwSignal::new(false);
    let attach = move |sink: AppRef| {
        apps.launch(sink);
        if let Some(instance) = apps.last_instance_of(sink) {
            pipeline::attach_sink(pipeline_id, instance.pid);
        }
        open.set(false);
        on_attach();
    };
    view! {
        <div class="workbench__attach">
            <button class="workbench__attach-btn" on:click=move |_| open.update(|o| *o = !*o)>
                "+ Attach sink"
            </button>
            <Show when=move || open.get()>
                <AppPicker apps=apps role=AppRole::Sink on_pick=attach />
            </Show>
        </div>
    }
}

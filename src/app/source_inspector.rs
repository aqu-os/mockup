//! A generic, non-bespoke viewer for any `Source` app's raw data block —
//! opened when a source is launched directly rather than through a sink.
//! Deliberately built off nothing but [`crate::os::kernel::DataBlock`]
//! and [`super::Action`], not any Contacts-specific knowledge, so it
//! demonstrates the source/manipulator/sink split is actually generic
//! rather than hardcoded per app.

use leptos::prelude::*;

use super::pipeline::{self, ActionButton, AppPicker};
use super::{AppRef, Apps};
use crate::os::kernel::{self as kernel, AppRole, Pid};

#[component]
pub fn SourceInspector(source: AppRef, pid: Pid) -> impl IntoView {
    let apps = expect_context::<Apps>();
    let attach_open = RwSignal::new(false);

    let block = move || kernel::block_owned_by(pid).and_then(kernel::read_block);

    let attach_sink = move |sink: AppRef| {
        let pipeline_id = pipeline::pipeline_for_source(pid).or_else(|| pipeline::create_pipeline(apps, source));
        let Some(pipeline_id) = pipeline_id else {
            return;
        };
        apps.launch(sink);
        if let Some(instance) = apps.last_instance_of(sink) {
            pipeline::attach_sink(pipeline_id, instance.pid);
        }
        attach_open.set(false);
    };

    view! {
        <div class="source-inspector">
            <div class="source-inspector__header">
                <span class="source-inspector__icon">{source.icon()}</span>
                <div>
                    <div class="source-inspector__title">{source.title()}</div>
                    <div class="source-inspector__desc">{source.description()}</div>
                </div>
            </div>

            <div class="source-inspector__actions">
                {move || {
                    let Some(block) = block() else {
                        return view! { <div class="source-inspector__empty">"Starting up\u{2026}"</div> }.into_any();
                    };
                    source
                        .actions()
                        .iter()
                        .map(|action| view! { <ActionButton source=source block_id=block.id owner=pid action=action /> })
                        .collect_view()
                        .into_any()
                }}
            </div>

            <div class="source-inspector__body">
                {move || match block() {
                    None => view! { <div class="source-inspector__empty">"No data yet."</div> }.into_any(),
                    Some(block) => render_table(&block.schema, &block.data).into_any(),
                }}
            </div>

            <div class="source-inspector__attach">
                <button class="source-inspector__attach-btn" on:click=move |_| attach_open.update(|o| *o = !*o)>
                    "Attach a sink"
                </button>
                <Show when=move || attach_open.get()>
                    <AppPicker apps=apps role=AppRole::Sink on_pick=attach_sink />
                </Show>
            </div>
        </div>
    }
}

/// Renders a block's rows as a plain table, headers taken from its
/// `schema` (a JSON array of field names — see `App::schema`). Doesn't
/// know or care what those fields mean.
fn render_table(schema: &serde_json::Value, data: &serde_json::Value) -> impl IntoView {
    let headers: Vec<String> =
        schema.as_array().map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let rows: Vec<serde_json::Value> = data.as_array().cloned().unwrap_or_default();

    if rows.is_empty() {
        return view! { <div class="source-inspector__empty">"No records."</div> }.into_any();
    }

    view! {
        <table class="source-inspector__table">
            <thead>
                <tr>{headers.iter().map(|h| view! { <th>{h.clone()}</th> }).collect_view()}</tr>
            </thead>
            <tbody>
                {rows
                    .into_iter()
                    .map(|row| {
                        let cells = headers
                            .iter()
                            .map(|h| {
                                let text = match row.get(h) {
                                    Some(serde_json::Value::String(s)) => s.clone(),
                                    Some(other) => other.to_string(),
                                    None => String::new(),
                                };
                                view! { <td>{text}</td> }
                            })
                            .collect_view();
                        view! { <tr>{cells}</tr> }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
        .into_any()
}

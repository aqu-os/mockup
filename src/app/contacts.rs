//! The Contacts sink: renders whatever a pipeline hands it (records from
//! `contacts_source`, optionally passed through `contacts_sort`/
//! `contacts_filter`) as a list/detail UI, and exposes the source's
//! declared actions (add/remove) as buttons. Owns no data of its own —
//! see `app::pipeline` for how the pipeline it reads from is built and
//! computed, and `contacts_source` for where the records actually live.

use std::collections::HashMap;

use leptos::prelude::*;

use super::pipeline::{self, AppPicker, PipelineChain, PipelineSummary};
use super::{App, AppRef, Apps};
use crate::os::kernel::{self as kernel, AppRole, Pid, PipelineId, SettingValue, SizeConstraints};

pub const ID: &str = "contacts";

pub struct ContactsApp;

impl App for ContactsApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Contacts"
    }
    fn description(&self) -> &'static str {
        "Keep track of your contacts."
    }
    fn icon(&self) -> &'static str {
        "\u{1F4C7}"
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (480.0, 360.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 380.0,
            min_height: 280.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn role(&self) -> Option<AppRole> {
        Some(AppRole::Sink)
    }
    fn render(&self, pid: Pid) -> AnyView {
        view! { <Contacts pid=pid /> }.into_any()
    }
}

/// A contact record, parsed from the JSON a pipeline hands the sink. Just
/// a convenience for the view below — nothing here is stored; the source
/// of truth is `contacts_source`'s data block.
#[derive(Clone, Debug, PartialEq)]
struct Contact {
    id: u64,
    name: String,
    role: String,
    email: String,
    phone: String,
}

impl Contact {
    fn from_json(v: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: v.get("id")?.as_u64()?,
            name: v.get("name")?.as_str()?.to_string(),
            role: v.get("role").and_then(|r| r.as_str()).unwrap_or_default().to_string(),
            email: v.get("email").and_then(|r| r.as_str()).unwrap_or_default().to_string(),
            phone: v.get("phone").and_then(|r| r.as_str()).unwrap_or_default().to_string(),
        })
    }
}

/// A stable, deterministic accent color per contact so avatars don't all
/// look the same without needing real photos.
fn avatar_hue(id: u64) -> u64 {
    (id.wrapping_mul(63)) % 360
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[component]
fn Contacts(pid: Pid) -> impl IntoView {
    let apps = expect_context::<Apps>();
    let editing_pipeline = RwSignal::new(false);

    let pipeline_id = move || pipeline::pipelines_for_sink(pid).into_iter().next().map(|p| p.id);

    let start_pipeline = move |source: AppRef| {
        if let Some(new_id) = pipeline::create_pipeline(apps, source) {
            pipeline::attach_sink(new_id, pid);
        }
    };

    view! {
        <div class="contacts-sink">
            {move || match pipeline_id() {
                None => {
                    view! {
                        <div class="contacts-sink__builder">
                            <div class="contacts-sink__builder-label">"Connect a data source"</div>
                            <AppPicker apps=apps role=AppRole::Source on_pick=start_pipeline />
                        </div>
                    }
                        .into_any()
                }
                Some(pipeline_id) if editing_pipeline.get() => {
                    view! {
                        <div class="contacts-sink__builder">
                            <div class="contacts-sink__builder-label">"Edit pipeline"</div>
                            <PipelineChain apps=apps pipeline_id=pipeline_id />
                            <button
                                class="contacts-sink__builder-done"
                                on:click=move |_| editing_pipeline.set(false)
                            >
                                "Done"
                            </button>
                        </div>
                    }
                        .into_any()
                }
                Some(pipeline_id) => {
                    view! { <ContactsList pipeline_id=pipeline_id editing_pipeline=editing_pipeline /> }
                        .into_any()
                }
            }}
        </div>
    }
}

#[component]
fn ContactsList(pipeline_id: PipelineId, editing_pipeline: RwSignal<bool>) -> impl IntoView {
    let selected: RwSignal<Option<u64>> = RwSignal::new(None);
    let new_name = RwSignal::new(String::new());
    let new_email = RwSignal::new(String::new());

    let contacts = move || {
        pipeline::compute(pipeline_id).as_array().cloned().unwrap_or_default().iter().filter_map(Contact::from_json).collect::<Vec<_>>()
    };

    let source_and_block = move || {
        let pipeline = kernel::read_pipeline(pipeline_id)?;
        let source = pipeline::resolve(pipeline.source)?;
        let block_id = kernel::block_owned_by(pipeline.source)?;
        Some((source, pipeline.source, block_id))
    };

    let add_contact = move || {
        let Some((source, owner, block_id)) = source_and_block() else {
            return;
        };
        let name = new_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let email = new_email.get_untracked().trim().to_string();
        let params = HashMap::from([("name", SettingValue::Text(name)), ("email", SettingValue::Text(email))]);
        source.invoke_action(block_id, owner, "add_contact", &params);
        new_name.set(String::new());
        new_email.set(String::new());
    };

    let remove_contact = move |id: u64| {
        let Some((source, owner, block_id)) = source_and_block() else {
            return;
        };
        let params = HashMap::from([("id", SettingValue::Number(id as f64))]);
        source.invoke_action(block_id, owner, "remove_contact", &params);
        selected.set(None);
    };

    view! {
        <div class="contacts-sink__header">
            <PipelineSummary pipeline_id=pipeline_id />
            <button class="contacts-sink__edit-btn" on:click=move |_| editing_pipeline.set(true)>
                "Edit pipeline"
            </button>
        </div>
        <div class="contacts">
            <div class="contacts__list">
                <For each=contacts key=|c| c.id let(contact)>
                    {
                        let id = contact.id;
                        let is_selected = move || selected.get() == Some(id);
                        view! {
                            <button
                                class="contact-item"
                                class:is-selected=is_selected
                                on:click=move |_| selected.set(Some(id))
                            >
                                <span
                                    class="contact-item__avatar"
                                    style:background=format!("hsl({} 55% 32%)", avatar_hue(id))
                                >
                                    {initials(&contact.name)}
                                </span>
                                <span class="contact-item__text">
                                    <span class="contact-item__name">{contact.name.clone()}</span>
                                    <span class="contact-item__role">{contact.role.clone()}</span>
                                </span>
                            </button>
                        }
                    }
                </For>

                <div class="contacts__add">
                    <input
                        class="contacts__input"
                        type="text"
                        placeholder="Name"
                        prop:value=move || new_name.get()
                        on:input=move |ev| new_name.set(event_target_value(&ev))
                    />
                    <input
                        class="contacts__input"
                        type="text"
                        placeholder="Email"
                        prop:value=move || new_email.get()
                        on:input=move |ev| new_email.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                add_contact();
                            }
                        }
                    />
                    <button class="contacts__add-btn" on:click=move |_| add_contact()>
                        "Add"
                    </button>
                </div>
            </div>

            <div class="contacts__detail">
                {move || {
                    let contact = selected.get().and_then(|id| contacts().into_iter().find(|c| c.id == id));
                    match contact {
                        None => {
                            view! {
                                <div class="contacts__empty">"Select a contact"</div>
                            }
                                .into_any()
                        }
                        Some(contact) => {
                            let id = contact.id;
                            view! {
                                <div class="contact-detail">
                                    <span
                                        class="contact-detail__avatar"
                                        style:background=format!("hsl({} 55% 32%)", avatar_hue(id))
                                    >
                                        {initials(&contact.name)}
                                    </span>
                                    <div class="contact-detail__name">{contact.name.clone()}</div>
                                    <div class="contact-detail__role">{contact.role.clone()}</div>
                                    <dl class="contact-detail__fields">
                                        <dt>"Email"</dt>
                                        <dd>{contact.email.clone()}</dd>
                                        <dt>"Phone"</dt>
                                        <dd>{contact.phone.clone()}</dd>
                                    </dl>
                                    <button
                                        class="contact-detail__remove"
                                        on:click=move |_| remove_contact(id)
                                    >
                                        "Remove contact"
                                    </button>
                                </div>
                            }
                                .into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}

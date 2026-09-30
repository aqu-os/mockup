//! Calculator: a small four-function calculator. Not pre-installed — it
//! only shows up once installed from the App Store, to give the store
//! something real to install.

use leptos::prelude::*;

use super::App;
use crate::os::kernel::{self as kernel, FieldKind, Owner, Pid, SettingField, SettingValue, SizeConstraints};

const ID: &str = "calculator";
const PRECISION_KEY: &str = "precision";

const SETTINGS: &[SettingField] = &[SettingField {
    key: PRECISION_KEY,
    label: "Decimal places",
    kind: FieldKind::Number { min: 0.0, max: 6.0, step: 1.0, suffix: "" },
    default: SettingValue::Number(2.0),
}];

pub struct CalculatorApp;

impl App for CalculatorApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Calculator"
    }
    fn description(&self) -> &'static str {
        "Basic arithmetic calculator."
    }
    fn icon(&self) -> &'static str {
        "\u{1F5A9}"
    }
    fn installed_by_default(&self) -> bool {
        false
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (260.0, 360.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 220.0,
            min_height: 320.0,
            max_width: Some(340.0),
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn settings_manifest(&self) -> &'static [SettingField] {
        SETTINGS
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <Calculator /> }.into_any()
    }
}

fn apply(acc: f64, op: char, val: f64) -> f64 {
    match op {
        '+' => acc + val,
        '-' => acc - val,
        '*' => acc * val,
        '/' => {
            if val != 0.0 {
                acc / val
            } else {
                f64::NAN
            }
        }
        _ => val,
    }
}

#[component]
fn Calculator() -> impl IntoView {
    let display = RwSignal::new("0".to_string());
    let accumulator: RwSignal<Option<f64>> = RwSignal::new(None);
    let pending_op: RwSignal<Option<char>> = RwSignal::new(None);
    let fresh = RwSignal::new(true);

    let format_result = move |v: f64| {
        let precision = kernel::get_setting_number(Owner::App(ID), PRECISION_KEY, 2.0);
        format!("{:.*}", precision.max(0.0) as usize, v)
    };

    let input_digit = move |d: char| {
        if fresh.get_untracked() {
            display.set(d.to_string());
            fresh.set(false);
        } else {
            display.update(|s| {
                if s == "0" {
                    *s = d.to_string();
                } else {
                    s.push(d);
                }
            });
        }
    };

    let input_decimal = move || {
        if fresh.get_untracked() {
            display.set("0.".to_string());
            fresh.set(false);
        } else {
            display.update(|s| {
                if !s.contains('.') {
                    s.push('.');
                }
            });
        }
    };

    let apply_op = move |op: char| {
        let current: f64 = display.get_untracked().parse().unwrap_or(0.0);
        let result = match (accumulator.get_untracked(), pending_op.get_untracked()) {
            (Some(acc), Some(prev_op)) => apply(acc, prev_op, current),
            _ => current,
        };
        accumulator.set(Some(result));
        pending_op.set(Some(op));
        display.set(format_result(result));
        fresh.set(true);
    };

    let equals = move || {
        let current: f64 = display.get_untracked().parse().unwrap_or(0.0);
        if let (Some(acc), Some(op)) = (accumulator.get_untracked(), pending_op.get_untracked()) {
            let result = apply(acc, op, current);
            display.set(format_result(result));
            accumulator.set(None);
            pending_op.set(None);
            fresh.set(true);
        }
    };

    let clear = move || {
        display.set("0".to_string());
        accumulator.set(None);
        pending_op.set(None);
        fresh.set(true);
    };

    view! {
        <div class="calc">
            <div class="calc__display">{move || display.get()}</div>
            <div class="calc__grid">
                <button class="calc__btn calc__btn--op" on:click=move |_| clear()>
                    "C"
                </button>
                <button class="calc__btn calc__btn--op" on:click=move |_| apply_op('/')>
                    "\u{00F7}"
                </button>
                <button class="calc__btn calc__btn--op" on:click=move |_| apply_op('*')>
                    "\u{00D7}"
                </button>
                <button class="calc__btn calc__btn--op" on:click=move |_| apply_op('-')>
                    "\u{2212}"
                </button>

                <button class="calc__btn" on:click=move |_| input_digit('7')>
                    "7"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('8')>
                    "8"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('9')>
                    "9"
                </button>
                <button class="calc__btn calc__btn--op calc__btn--tall" on:click=move |_| apply_op('+')>
                    "+"
                </button>

                <button class="calc__btn" on:click=move |_| input_digit('4')>
                    "4"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('5')>
                    "5"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('6')>
                    "6"
                </button>

                <button class="calc__btn" on:click=move |_| input_digit('1')>
                    "1"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('2')>
                    "2"
                </button>
                <button class="calc__btn" on:click=move |_| input_digit('3')>
                    "3"
                </button>
                <button class="calc__btn calc__btn--accent calc__btn--tall" on:click=move |_| equals()>
                    "="
                </button>

                <button class="calc__btn calc__btn--wide" on:click=move |_| input_digit('0')>
                    "0"
                </button>
                <button class="calc__btn" on:click=move |_| input_decimal()>
                    "."
                </button>
            </div>
        </div>
    }
}

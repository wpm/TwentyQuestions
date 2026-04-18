use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::store::{load_store, store_set_string, STORE_PATH};

type EscListener = Rc<RefCell<Option<Closure<dyn Fn(KeyboardEvent)>>>>;

pub const NATS_URL_KEY: &str = "nats_url";
pub const MODEL_KEY: &str = "model";

pub const DEFAULT_NATS_URL: &str = "nats://localhost:4222";
pub const DEFAULT_MODEL: &str = "claude-opus-4-7";

pub const MODELS: &[(&str, &str)] = &[
    ("claude-opus-4-7", "Claude Opus 4.7"),
    ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
    ("claude-haiku-4-5-20251001", "Claude Haiku 4.5"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_nats_url_is_valid_nats_scheme() {
        assert!(DEFAULT_NATS_URL.starts_with("nats://"));
        let host_port = DEFAULT_NATS_URL.trim_start_matches("nats://");
        let (host, port_str) = host_port.rsplit_once(':').expect("expected host:port");
        assert!(!host.is_empty());
        port_str.parse::<u16>().expect("port must be a valid u16");
    }

    #[test]
    fn default_model_is_in_models_table() {
        assert!(
            MODELS.iter().any(|(id, _)| *id == DEFAULT_MODEL),
            "{DEFAULT_MODEL} not found in MODELS"
        );
    }

    #[test]
    fn models_table_entries_are_non_empty() {
        for (id, label) in MODELS {
            assert!(!id.is_empty(), "model id must not be empty");
            assert!(!label.is_empty(), "model label must not be empty");
        }
    }
}

#[component]
pub fn SettingsModal(
    show: RwSignal<bool>,
    nats_url: RwSignal<String>,
    model: RwSignal<String>,
) -> impl IntoView {
    let draft_nats_url = RwSignal::new(String::new());
    let draft_model = RwSignal::new(String::new());

    Effect::new(move || {
        if show.get() {
            // get_untracked so a concurrent save doesn't reset an in-progress edit
            draft_nats_url.set(nats_url.get_untracked());
            draft_model.set(model.get_untracked());
        }
    });

    let changed =
        move || draft_nats_url.get() != nats_url.get() || draft_model.get() != model.get();

    let apply = move |_: web_sys::MouseEvent| {
        let new_url = draft_nats_url.get_untracked();
        let new_model = draft_model.get_untracked();
        nats_url.set(new_url.clone());
        model.set(new_model.clone());
        spawn_local(async move {
            if let Some(rid) = load_store(STORE_PATH).await {
                store_set_string(rid, NATS_URL_KEY, &new_url).await;
                store_set_string(rid, MODEL_KEY, &new_model).await;
            }
        });
        show.set(false);
    };

    let close = move |_: web_sys::MouseEvent| {
        show.set(false);
    };

    // The Rc<RefCell<...>> holds the closure across effect re-runs without leaking.
    let esc_listener: EscListener = Rc::new(RefCell::new(None));

    Effect::new(move || {
        let doc = web_sys::window()
            .and_then(|w| w.document())
            .expect("document");

        if let Some(cb) = esc_listener.borrow_mut().take() {
            let _ = doc.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }

        if show.get() {
            let cb = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
                if ev.key() == "Escape" {
                    show.set(false);
                }
            }) as Box<dyn Fn(KeyboardEvent)>);
            let _ = doc.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            *esc_listener.borrow_mut() = Some(cb);
        }
    });

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                <div class="relative bg-white dark:bg-gray-900 rounded-lg shadow-xl w-full max-w-md mx-4 p-6">
                    <div class="flex items-center justify-between mb-6">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">"Settings"</h2>
                        <button
                            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 p-1 rounded"
                            aria-label="Close"
                            on:click=close
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "NATS URL"
                        </label>
                        <input
                            type="text"
                            spellcheck="false"
                            autocomplete="off"
                            class="w-full border border-gray-300 dark:border-gray-600 rounded px-3 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            prop:value=move || draft_nats_url.get()
                            on:input=move |ev| draft_nats_url.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="mb-6">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "LLM Model"
                        </label>
                        <select
                            class="w-full border border-gray-300 dark:border-gray-600 rounded px-3 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            on:change=move |ev| draft_model.set(event_target_value(&ev))
                        >
                            {MODELS.iter().map(|(id, label)| {
                                let id = *id;
                                let label = *label;
                                view! {
                                    <option value=id selected=move || draft_model.get() == id>
                                        {label}
                                    </option>
                                }
                            }).collect_view()}
                        </select>
                    </div>

                    <div class="flex justify-end gap-3">
                        <button
                            class="px-4 py-2 text-sm font-medium rounded border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
                            on:click=close
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-4 py-2 text-sm font-medium rounded bg-blue-600 hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed text-white"
                            disabled=move || !changed()
                            on:click=apply
                        >
                            "Apply"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::store::{load_store, store_get_string, store_set_string};

pub const OBJECT_SUGGESTIONS: &[&str] = &["elephant", "wine glass", "smile", "umbrella", "doctor"];

const STORE_PATH: &str = "settings.json";
const THEME_KEY: &str = "theme";

#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

fn apply_theme(theme: Theme) {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element());
    if let Some(el) = doc {
        match theme {
            Theme::Dark => {
                let _ = el.class_list().add_1("dark");
            }
            Theme::Light => {
                let _ = el.class_list().remove_1("dark");
            }
        }
    }
}

fn system_prefers_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .map(|mq| mq.matches())
        .unwrap_or(false)
}

#[component]
pub fn Toolbar(
    object: RwSignal<String>,
    players: RwSignal<u32>,
    theme: RwSignal<Theme>,
) -> impl IntoView {
    // Load persisted theme on mount, falling back to system preference
    spawn_local(async move {
        if let Some(rid) = load_store(STORE_PATH).await {
            if let Some(saved) = store_get_string(rid, THEME_KEY).await {
                if let Some(t) = Theme::from_str(&saved) {
                    theme.set(t);
                    apply_theme(t);
                    return;
                }
            }
        }
        let t = if system_prefers_dark() {
            Theme::Dark
        } else {
            Theme::Light
        };
        theme.set(t);
        apply_theme(t);
    });

    let toggle_theme = move |_: web_sys::MouseEvent| {
        let next = match theme.get() {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        theme.set(next);
        apply_theme(next);
        spawn_local(async move {
            if let Some(rid) = load_store(STORE_PATH).await {
                store_set_string(rid, THEME_KEY, next.as_str()).await;
            }
        });
    };

    view! {
        <div class="flex items-center gap-4 px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900">
            // Object combobox
            <div class="flex items-center gap-2">
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Object"</label>
                <input
                    list="object-suggestions"
                    spellcheck="false"
                    autocomplete="off"
                    class="border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 w-36"
                    prop:value=move || object.get()
                    on:input=move |ev| object.set(event_target_value(&ev))
                />
                <datalist id="object-suggestions">
                    {OBJECT_SUGGESTIONS.iter().map(|s| view! {
                        <option value=*s />
                    }).collect_view()}
                </datalist>
            </div>

            // Players numeric select
            <div class="flex items-center gap-2">
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Players"</label>
                <select
                    class="border border-gray-300 dark:border-gray-600 rounded px-2 py-1 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                    on:change=move |ev| {
                        if let Ok(n) = event_target_value(&ev).parse::<u32>() {
                            players.set(n);
                        }
                    }
                >
                    {(1u32..=10).map(|n| view! {
                        <option value=n.to_string() selected=move || players.get() == n>
                            {n.to_string()}
                        </option>
                    }).collect_view()}
                </select>
            </div>

            <div class="flex-1" />

            // Start button
            <button class="px-3 py-1 text-sm font-medium rounded bg-green-600 hover:bg-green-700 text-white">
                "Start"
            </button>

            // Stop button
            <button class="px-3 py-1 text-sm font-medium rounded bg-red-600 hover:bg-red-700 text-white">
                "Stop"
            </button>

            // Theme toggle — sits in the title bar row, right of the buttons
            <button
                class="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                on:click=toggle_theme
                aria-label="Toggle light/dark mode"
            >
                {move || match theme.get() {
                    Theme::Light => view! { <span class="text-lg">"☀️"</span> },
                    Theme::Dark => view! { <span class="text-lg">"🌙"</span> },
                }}
            </button>
        </div>
    }
}

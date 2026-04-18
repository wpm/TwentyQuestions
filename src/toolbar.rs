use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::nats::is_valid_nats_subject;
use crate::store::{load_store, store_set_string, STORE_PATH};

pub const OBJECT_SUGGESTIONS: &[&str] = &["elephant", "wine glass", "smile", "umbrella", "doctor"];

pub const THEME_KEY: &str = "theme";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn apply(theme: Theme) {
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

    pub fn from_system() -> Self {
        let prefers_dark = web_sys::window()
            .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
            .map(|mq| mq.matches())
            .unwrap_or(false);
        if prefers_dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_from_str_rejects_unknown() {
        assert_eq!(Theme::from_str(""), None);
        assert_eq!(Theme::from_str("Light"), None);
        assert_eq!(Theme::from_str("Dark"), None);
        assert_eq!(Theme::from_str("auto"), None);
    }

    #[test]
    fn theme_round_trips_through_str() {
        assert_eq!(Theme::from_str(Theme::Light.as_str()), Some(Theme::Light));
        assert_eq!(Theme::from_str(Theme::Dark.as_str()), Some(Theme::Dark));
    }
}

#[component]
pub fn Toolbar(
    object: RwSignal<String>,
    players: RwSignal<u32>,
    theme: RwSignal<Theme>,
    on_settings: Callback<()>,
    topic: RwSignal<String>,
    on_topic_change: Callback<String>,
) -> impl IntoView {
    let toggle_theme = move |_: web_sys::MouseEvent| {
        let next = match theme.get() {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        theme.set(next);
        Theme::apply(next);
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

            // Topic input
            <div class="flex items-center gap-2">
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Topic"</label>
                <input
                    type="text"
                    spellcheck="false"
                    autocomplete="off"
                    class="border rounded px-2 py-1 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 w-44"
                    class=(
                        "border-red-500 dark:border-red-500",
                        move || !is_valid_nats_subject(&topic.get()),
                    )
                    class=(
                        "border-gray-300 dark:border-gray-600",
                        move || is_valid_nats_subject(&topic.get()),
                    )
                    prop:value=move || topic.get()
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        if is_valid_nats_subject(&val) {
                            on_topic_change.run(val);
                        } else {
                            topic.set(val);
                        }
                    }
                />
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

            <button
                class="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
                on:click=move |_| on_settings.run(())
                aria-label="Open settings"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
            </button>

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

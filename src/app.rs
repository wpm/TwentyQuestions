use leptos::prelude::*;

use crate::chat_panel::ChatPanel;
use crate::toolbar::{Theme, Toolbar};

#[component]
pub fn App() -> impl IntoView {
    let object = RwSignal::new(String::new());
    let players = RwSignal::new(1u32);
    let theme = RwSignal::new(Theme::Light);

    view! {
        <main class="flex flex-col h-screen bg-gray-50 dark:bg-gray-950 text-gray-900 dark:text-gray-100">
            <Toolbar object=object players=players theme=theme />
            <div class="flex flex-col flex-1 overflow-hidden p-4">
                <ChatPanel messages=vec![] />
            </div>
        </main>
    }
}

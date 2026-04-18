use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

// Fixed color palette indexed by participant. Index 0 = host, 1..N = players.
const COLORS: &[&str] = &[
    "#d1fae5", // host   — green-100
    "#dbeafe", // p1     — blue-100
    "#fce7f3", // p2     — pink-100
    "#fef9c3", // p3     — yellow-100
    "#ede9fe", // p4     — violet-100
    "#ffedd5", // p5     — orange-100
    "#cffafe", // p6     — cyan-100
    "#f3f4f6", // p7     — gray-100
    "#fef2f2", // p8     — red-100
    "#f0fdf4", // p9     — green-50
    "#eff6ff", // p10    — blue-50
];

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub id: u64,
    pub sender: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            sender: sender.into(),
            content: content.into(),
        }
    }

    /// Returns the background color for this sender.
    pub fn color(&self) -> &'static str {
        speaker_color(&self.sender)
    }

    /// Host is left-aligned; players are right-aligned.
    pub fn is_left(&self) -> bool {
        self.sender == "host"
    }
}

/// Deterministic color assignment: "host" → index 0, "player N" → index N.
pub fn speaker_color(sender: &str) -> &'static str {
    let index = if sender == "host" {
        0
    } else if let Some(rest) = sender.strip_prefix("player ") {
        rest.parse::<usize>().unwrap_or(0)
    } else {
        0
    };
    COLORS[index.min(COLORS.len() - 1)]
}

#[component]
fn ChatBalloon(msg: ChatMessage) -> impl IntoView {
    let color = msg.color();
    let is_left = msg.is_left();
    let row_class = if is_left {
        "flex justify-start"
    } else {
        "flex justify-end"
    };
    let bubble_class = if is_left {
        "rounded-2xl rounded-tl-sm"
    } else {
        "rounded-2xl rounded-tr-sm"
    };
    let style = format!("background-color: {color};");
    let sender = msg.sender.clone();
    let content = msg.content.clone();

    view! {
        <div class=row_class>
            <div
                class=format!("max-w-[60%] px-4 py-2 shadow-sm text-sm break-words {bubble_class}")
                style=style
            >
                <div class="font-semibold text-xs mb-1 text-gray-600">{sender}</div>
                <div>{content}</div>
            </div>
        </div>
    }
}

#[component]
pub fn ChatPanel(messages: RwSignal<Vec<ChatMessage>>) -> impl IntoView {
    let scroll_ref = NodeRef::<leptos::html::Div>::new();

    // Scroll to bottom whenever messages change.
    Effect::new(move || {
        let _ = messages.get();
        if let Some(el) = scroll_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });

    view! {
        <div
            node_ref=scroll_ref
            class="flex flex-col flex-1 gap-2 overflow-y-auto p-3 \
                   border border-gray-200 dark:border-gray-700 rounded-lg"
        >
            <For
                each=move || messages.get()
                key=|msg| msg.id
                children=|msg| view! { <ChatBalloon msg=msg /> }
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_is_left_aligned() {
        let msg = ChatMessage::new("host", "Is it alive?");
        assert!(msg.is_left());
    }

    #[test]
    fn player_is_right_aligned() {
        let msg = ChatMessage::new("player 1", "My guess is a dolphin.");
        assert!(!msg.is_left());
    }

    #[test]
    fn host_gets_index_zero_color() {
        assert_eq!(speaker_color("host"), COLORS[0]);
    }

    #[test]
    fn player_gets_indexed_color() {
        assert_eq!(speaker_color("player 1"), COLORS[1]);
        assert_eq!(speaker_color("player 3"), COLORS[3]);
    }

    #[test]
    fn color_index_clamps_to_palette_size() {
        assert_eq!(speaker_color("player 999"), *COLORS.last().unwrap());
    }

    #[test]
    fn unknown_sender_falls_back_to_index_zero() {
        assert_eq!(speaker_color("mystery"), COLORS[0]);
    }
}

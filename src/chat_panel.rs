#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum BalloonAlignment {
    Left,
    Right,
}

#[derive(Clone, PartialEq)]
pub struct ChatMessage {
    pub text: String,
    pub color: String,
    pub alignment: BalloonAlignment,
}

#[allow(dead_code)]
impl ChatMessage {
    pub fn new(
        text: impl Into<String>,
        color: impl Into<String>,
        alignment: BalloonAlignment,
    ) -> Self {
        Self {
            text: text.into(),
            color: color.into(),
            alignment,
        }
    }
}

use leptos::prelude::*;

#[component]
pub fn ChatBalloon(text: String, color: String, alignment: BalloonAlignment) -> impl IntoView {
    let row_class = match alignment {
        BalloonAlignment::Left => "flex justify-start",
        BalloonAlignment::Right => "flex justify-end",
    };
    let style = format!("background-color: {};", color);

    view! {
        <div class=row_class>
            <div class="max-w-[60%] px-4 py-2 rounded-2xl shadow-sm text-sm break-words" style=style>
                {text}
            </div>
        </div>
    }
}

#[component]
pub fn ChatPanel(messages: Vec<ChatMessage>) -> impl IntoView {
    view! {
        <div class="flex flex-col flex-1 gap-2 overflow-y-auto p-3 border border-gray-200 rounded-lg">
            {messages.into_iter().map(|msg| {
                view! {
                    <ChatBalloon
                        text=msg.text
                        color=msg.color
                        alignment=msg.alignment
                    />
                }
            }).collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_new_stores_fields() {
        let msg = ChatMessage::new("hello", "#ff0000", BalloonAlignment::Left);
        assert_eq!(msg.text, "hello");
        assert_eq!(msg.color, "#ff0000");
        assert_eq!(msg.alignment, BalloonAlignment::Left);
    }

    #[test]
    fn twenty_questions_conversation() {
        let messages = [
            ChatMessage::new(
                "Hello! What are you thinking of?",
                "#d1e8ff",
                BalloonAlignment::Left,
            ),
            ChatMessage::new(
                "I'm thinking of an animal.",
                "#d4f7d4",
                BalloonAlignment::Right,
            ),
            ChatMessage::new(
                "Is it larger than a house cat?",
                "#d1e8ff",
                BalloonAlignment::Left,
            ),
        ];
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].alignment, BalloonAlignment::Left);
        assert_eq!(messages[1].alignment, BalloonAlignment::Right);
        assert_eq!(messages[2].alignment, BalloonAlignment::Left);
    }
}

use async_nats::Client as NatsClient;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::message::Message as GameMessage;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 1024;
const MAX_TRANSCRIPT: usize = 200;

static HOST_SYSTEM_PROMPT: &str = include_str!("../prompts/host.txt");
static PLAYER_SYSTEM_PROMPT: &str = include_str!("../prompts/player.txt");

// ── Tool definitions ──────────────────────────────────────────────────────────

static HOST_TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!([
        {
            "name": "speak",
            "description": "Say something to all players.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "What you want to say." }
                },
                "required": ["message"]
            }
        },
        {
            "name": "leave_game",
            "description": "Leave the game. Call this after announcing it is over.",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }
    ])
});

static PLAYER_TOOLS: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!([
        {
            "name": "speak",
            "description": "Say something to everyone — ask a question, make a \
                            comment, or declare your guess.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "What you want to say." }
                },
                "required": ["message"]
            }
        },
        {
            "name": "leave_game",
            "description": "Leave the game. Call this only after the host declares \
                            it over.",
            "input_schema": { "type": "object", "properties": {}, "required": [] }
        }
    ])
});

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    content: Value,
}

impl ChatMessage {
    fn text(role: &str, name: Option<String>, content: &str) -> Self {
        Self {
            role: role.to_string(),
            name,
            content: json!(content),
        }
    }

    fn tool_result(tool_use_id: &str, content: &str) -> Self {
        Self {
            role: "user".to_string(),
            name: None,
            content: json!([{
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content
            }]),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ApiRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    tools: Value,
    messages: &'a [ChatMessage],
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiResponse {
    stop_reason: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ContentBlock {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<Value>,
}

// ── Backend trait ─────────────────────────────────────────────────────────────

/// Abstracts the API call so tests can inject deterministic responses.
pub trait Backend: Send {
    fn complete<'a>(
        &'a mut self,
        request: &'a ApiRequest<'a>,
    ) -> impl std::future::Future<Output = Result<ApiResponse, String>> + Send + 'a;
}

// ── AnthropicBackend ──────────────────────────────────────────────────────────

pub struct AnthropicBackend {
    http: Client,
    api_key: String,
}

impl AnthropicBackend {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
        }
    }
}

impl Backend for AnthropicBackend {
    async fn complete<'a>(
        &'a mut self,
        request: &'a ApiRequest<'a>,
    ) -> Result<ApiResponse, String> {
        let response = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error {status}: {body}"));
        }

        response
            .json::<ApiResponse>()
            .await
            .map_err(|e| e.to_string())
    }
}

// ── ScriptedBackend ───────────────────────────────────────────────────────────

/// Test backend that returns pre-canned `ApiResponse` values in sequence.
/// Panics if exhausted before the conversation ends.
#[cfg_attr(not(test), allow(dead_code))]
pub struct ScriptedBackend {
    responses: std::collections::VecDeque<ApiResponse>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ScriptedBackend {
    pub fn new(responses: Vec<ApiResponse>) -> Self {
        Self {
            responses: responses.into(),
        }
    }

    /// Convenience: a response that terminates with `end_turn` (silence).
    pub fn silent() -> ApiResponse {
        ApiResponse {
            stop_reason: "end_turn".to_string(),
            content: vec![],
        }
    }

    /// Convenience: a response that calls `speak` with the given text.
    pub fn speak(text: &str) -> ApiResponse {
        ApiResponse {
            stop_reason: "tool_use".to_string(),
            content: vec![ContentBlock {
                kind: "tool_use".to_string(),
                text: None,
                id: Some("id_speak".to_string()),
                name: Some("speak".to_string()),
                input: Some(json!({ "message": text })),
            }],
        }
    }

    /// Convenience: a response that calls `leave_game`.
    pub fn leave() -> ApiResponse {
        ApiResponse {
            stop_reason: "tool_use".to_string(),
            content: vec![ContentBlock {
                kind: "tool_use".to_string(),
                text: None,
                id: Some("id_leave".to_string()),
                name: Some("leave_game".to_string()),
                input: Some(json!({})),
            }],
        }
    }
}

impl Backend for ScriptedBackend {
    async fn complete<'a>(
        &'a mut self,
        _request: &'a ApiRequest<'a>,
    ) -> Result<ApiResponse, String> {
        self.responses
            .pop_front()
            .ok_or_else(|| "ScriptedBackend: no more responses".to_string())
    }
}

// ── Transcript ────────────────────────────────────────────────────────────────

struct TranscriptEntry {
    sender: String,
    content: String,
    timestamp: DateTime<Utc>,
}

impl TranscriptEntry {
    fn format(&self) -> String {
        let ts = self.timestamp.format("%H:%M:%S");
        format!("[{ts}] {}: {}", self.sender, self.content)
    }
}

// ── Public result type ────────────────────────────────────────────────────────

pub enum ThinkResult {
    Silent,
    Left,
}

// ── Role ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Role {
    Host,
    Player { index: u32 },
}

impl Role {
    pub fn name(&self) -> String {
        match self {
            Role::Host => "host".to_string(),
            Role::Player { index } => format!("player {}", index),
        }
    }

    fn tools(&self) -> Value {
        match self {
            Role::Host => HOST_TOOLS.clone(),
            Role::Player { .. } => PLAYER_TOOLS.clone(),
        }
    }
}

// ── LanguageModel ─────────────────────────────────────────────────────────────

pub struct LanguageModel<B: Backend = AnthropicBackend> {
    backend: B,
    model: String,
    role: Role,
    system_prompt: String,
    topic: String,
    transcript: std::collections::VecDeque<TranscriptEntry>,
}

impl LanguageModel<AnthropicBackend> {
    pub fn new_host(api_key: &str, model: impl Into<String>, object: &str, topic: &str) -> Self {
        let system_prompt = HOST_SYSTEM_PROMPT.replace("{object}", object);
        Self::with_backend(
            AnthropicBackend::new(api_key),
            model,
            Role::Host,
            system_prompt,
            topic,
        )
    }

    pub fn new_player(api_key: &str, model: impl Into<String>, index: u32, topic: &str) -> Self {
        Self::with_backend(
            AnthropicBackend::new(api_key),
            model,
            Role::Player { index },
            PLAYER_SYSTEM_PROMPT.to_string(),
            topic,
        )
    }
}

impl<B: Backend> LanguageModel<B> {
    pub fn with_backend(
        backend: B,
        model: impl Into<String>,
        role: Role,
        system_prompt: String,
        topic: &str,
    ) -> Self {
        Self {
            backend,
            model: model.into(),
            role,
            system_prompt,
            topic: topic.to_string(),
            transcript: std::collections::VecDeque::new(),
        }
    }

    pub fn name(&self) -> String {
        self.role.name()
    }

    /// Record an incoming game message. Returns `false` if this entity sent it.
    pub fn record_message(&mut self, msg: &GameMessage) -> bool {
        if msg.sender == self.role.name() {
            return false;
        }
        self.append_transcript(TranscriptEntry {
            sender: msg.sender.clone(),
            content: msg.content.clone(),
            timestamp: msg.timestamp,
        });
        true
    }

    fn append_transcript(&mut self, entry: TranscriptEntry) {
        if self.transcript.len() >= MAX_TRANSCRIPT {
            self.transcript.pop_front();
        }
        self.transcript.push_back(entry);
    }

    /// Decide whether to speak, stay silent, or leave.
    pub async fn think(&mut self, nats: &NatsClient, subject: &str) -> Result<ThinkResult, String> {
        let user_turn = self.build_user_turn();
        let mut api_history = vec![ChatMessage::text("user", None, &user_turn)];

        loop {
            let request = ApiRequest {
                model: &self.model,
                max_tokens: MAX_TOKENS,
                system: &self.system_prompt,
                tools: self.role.tools(),
                messages: &api_history,
            };

            let api_response = self.backend.complete(&request).await?;

            let assistant_content: Value = api_response
                .content
                .iter()
                .map(|b| match b.kind.as_str() {
                    "text" => json!({ "type": "text", "text": b.text.as_deref().unwrap_or("") }),
                    "tool_use" => json!({
                        "type": "tool_use",
                        "id": b.id,
                        "name": b.name,
                        "input": b.input.as_ref().unwrap_or(&json!({}))
                    }),
                    _ => json!({ "type": b.kind }),
                })
                .collect();
            api_history.push(ChatMessage {
                role: "assistant".to_string(),
                name: Some(self.role.name()),
                content: assistant_content,
            });

            if api_response.stop_reason != "tool_use" {
                return Ok(ThinkResult::Silent);
            }

            let mut left = false;
            for block in &api_response.content {
                if block.kind != "tool_use" {
                    continue;
                }
                let tool_id = block.id.as_deref().unwrap_or("");
                let tool_name = block.name.as_deref().unwrap_or("");
                let input = block.input.as_ref().unwrap_or(&Value::Null);

                let result = match tool_name {
                    "speak" => {
                        let text = input["message"].as_str().unwrap_or("").to_string();
                        publish_message(nats, &self.topic, &self.role.name(), &text).await;
                        self.append_transcript(TranscriptEntry {
                            sender: self.role.name(),
                            content: text.clone(),
                            timestamp: Utc::now(),
                        });
                        tracing::info!("{} spoke: {text}", self.role.name());
                        "Message sent.".to_string()
                    }
                    "leave_game" => {
                        publish_stop(nats, subject).await;
                        left = true;
                        "You have left the game.".to_string()
                    }
                    unknown => format!("Unknown tool: {unknown}"),
                };

                api_history.push(ChatMessage::tool_result(tool_id, &result));
            }

            if left {
                return Ok(ThinkResult::Left);
            }
        }
    }

    fn build_user_turn(&self) -> String {
        if self.transcript.is_empty() {
            return "The game has just started. Decide whether to speak or stay silent."
                .to_string();
        }
        let lines: Vec<String> = self.transcript.iter().map(|e| e.format()).collect();
        format!(
            "Transcript so far:\n\n{}\n\nDecide whether to speak or stay silent.",
            lines.join("\n")
        )
    }

    #[cfg(test)]
    pub fn transcript_len(&self) -> usize {
        self.transcript.len()
    }

    #[cfg(test)]
    pub fn user_turn(&self) -> String {
        self.build_user_turn()
    }
}

// ── NATS helpers ──────────────────────────────────────────────────────────────

async fn publish_message(nats: &NatsClient, subject: &str, sender: &str, content: &str) {
    use free_agent::Envelope;
    let msg = GameMessage::new(sender, content);
    let Ok(payload) = serde_json::to_vec(&Envelope::Message(msg)) else {
        return;
    };
    let _ = nats.publish(subject.to_string(), payload.into()).await;
    let _ = nats.flush().await;
}

async fn publish_stop(nats: &NatsClient, subject: &str) {
    use free_agent::{Control, Envelope};
    let Ok(payload) = serde_json::to_vec(&Envelope::<GameMessage>::Control(Control::Stop)) else {
        return;
    };
    let _ = nats.publish(subject.to_string(), payload.into()).await;
    let _ = nats.flush().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn scripted_host(responses: Vec<ApiResponse>) -> LanguageModel<ScriptedBackend> {
        LanguageModel::with_backend(
            ScriptedBackend::new(responses),
            "test-model",
            Role::Host,
            HOST_SYSTEM_PROMPT.replace("{object}", "elephant"),
            "twenty-questions",
        )
    }

    fn scripted_player(index: u32, responses: Vec<ApiResponse>) -> LanguageModel<ScriptedBackend> {
        LanguageModel::with_backend(
            ScriptedBackend::new(responses),
            "test-model",
            Role::Player { index },
            PLAYER_SYSTEM_PROMPT.to_string(),
            "twenty-questions",
        )
    }

    fn game_msg(sender: &str, content: &str) -> GameMessage {
        GameMessage::new(sender, content)
    }

    // ── Role::name ────────────────────────────────────────────────────────────

    #[test]
    fn host_name() {
        assert_eq!(scripted_host(vec![]).name(), "host");
    }

    #[test]
    fn player_name() {
        assert_eq!(scripted_player(1, vec![]).name(), "player 1");
        assert_eq!(scripted_player(7, vec![]).name(), "player 7");
    }

    // ── System prompt interpolation ───────────────────────────────────────────

    #[test]
    fn host_prompt_contains_object() {
        let lm = scripted_host(vec![]);
        assert!(lm.system_prompt.contains("elephant"));
        assert!(!lm.system_prompt.contains("{object}"));
    }

    // ── record_message ────────────────────────────────────────────────────────

    #[test]
    fn records_message_from_other_sender() {
        let mut lm = scripted_host(vec![]);
        assert!(lm.record_message(&game_msg("player 1", "Is it alive?")));
        assert_eq!(lm.transcript_len(), 1);
    }

    #[test]
    fn ignores_own_message() {
        let mut lm = scripted_host(vec![]);
        assert!(!lm.record_message(&game_msg("host", "Yes, it is!")));
        assert_eq!(lm.transcript_len(), 0);
    }

    #[test]
    fn player_ignores_own_message() {
        let mut lm = scripted_player(2, vec![]);
        assert!(!lm.record_message(&game_msg("player 2", "Is it an animal?")));
        assert_eq!(lm.transcript_len(), 0);
    }

    #[test]
    fn player_records_message_from_host() {
        let mut lm = scripted_player(1, vec![]);
        assert!(lm.record_message(&game_msg("host", "Yes!")));
        assert_eq!(lm.transcript_len(), 1);
    }

    #[test]
    fn transcript_caps_at_max() {
        let mut lm = scripted_host(vec![]);
        for i in 0..=MAX_TRANSCRIPT {
            lm.record_message(&game_msg("player 1", &format!("message {i}")));
        }
        assert_eq!(lm.transcript_len(), MAX_TRANSCRIPT);
    }

    #[test]
    fn transcript_cap_drops_oldest() {
        let mut lm = scripted_host(vec![]);
        for i in 0..=MAX_TRANSCRIPT {
            lm.record_message(&game_msg("player 1", &format!("message {i}")));
        }
        let turn = lm.user_turn();
        assert!(
            !turn.contains("message 0"),
            "oldest entry should have been dropped"
        );
        assert!(turn.contains(&format!("message {MAX_TRANSCRIPT}")));
    }

    // ── build_user_turn ───────────────────────────────────────────────────────

    #[test]
    fn empty_transcript_produces_start_prompt() {
        assert!(scripted_host(vec![]).user_turn().contains("just started"));
    }

    #[test]
    fn user_turn_contains_sender_and_content() {
        let mut lm = scripted_host(vec![]);
        lm.record_message(&game_msg("player 1", "Is it an animal?"));
        let turn = lm.user_turn();
        assert!(turn.contains("player 1"));
        assert!(turn.contains("Is it an animal?"));
    }

    #[test]
    fn user_turn_ends_with_decide_prompt() {
        let mut lm = scripted_host(vec![]);
        lm.record_message(&game_msg("player 1", "hello"));
        assert!(lm
            .user_turn()
            .ends_with("Decide whether to speak or stay silent."));
    }

    #[test]
    fn user_turn_preserves_message_order() {
        let mut lm = scripted_host(vec![]);
        lm.record_message(&game_msg("player 1", "first"));
        lm.record_message(&game_msg("player 2", "second"));
        let turn = lm.user_turn();
        assert!(turn.find("first").unwrap() < turn.find("second").unwrap());
    }

    // ── TranscriptEntry::format ───────────────────────────────────────────────

    #[test]
    fn transcript_entry_format_includes_timestamp_sender_content() {
        let entry = TranscriptEntry {
            sender: "player 3".to_string(),
            content: "Is it bigger than a car?".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 14, 5, 9).unwrap(),
        };
        let formatted = entry.format();
        assert!(formatted.contains("[14:05:09]"));
        assert!(formatted.contains("player 3"));
        assert!(formatted.contains("Is it bigger than a car?"));
    }

    // ── think() with ScriptedBackend ──────────────────────────────────────────

    #[tokio::test]
    async fn think_silent_when_no_tool_called() {
        let nats = async_nats::connect("nats://localhost:4222").await;
        let Ok(nats) = nats else { return }; // skip if no local NATS

        let mut lm = scripted_host(vec![ScriptedBackend::silent()]);
        lm.record_message(&game_msg("player 1", "Is it an animal?"));
        let result = lm.think(&nats, "twenty-questions").await.unwrap();
        assert!(matches!(result, ThinkResult::Silent));
    }

    #[tokio::test]
    async fn think_left_when_leave_game_called() {
        let nats = async_nats::connect("nats://localhost:4222").await;
        let Ok(nats) = nats else { return };

        let mut lm = scripted_host(vec![ScriptedBackend::leave()]);
        let result = lm.think(&nats, "twenty-questions").await.unwrap();
        assert!(matches!(result, ThinkResult::Left));
    }

    #[tokio::test]
    async fn think_speak_adds_to_transcript() {
        let nats = async_nats::connect("nats://localhost:4222").await;
        let Ok(nats) = nats else { return };

        let mut lm = scripted_host(vec![
            ScriptedBackend::speak("Yes, it is alive!"),
            ScriptedBackend::silent(), // tool result → model continues → silent
        ]);
        lm.record_message(&game_msg("player 1", "Is it alive?"));
        let _ = lm.think(&nats, "twenty-questions").await.unwrap();
        // Own utterance should be in transcript now
        assert_eq!(lm.transcript_len(), 2);
        assert!(lm.user_turn().contains("Yes, it is alive!"));
    }

    #[tokio::test]
    async fn think_returns_error_on_exhausted_backend() {
        let nats = async_nats::connect("nats://localhost:4222").await;
        let Ok(nats) = nats else { return };

        let mut lm = scripted_host(vec![]); // no responses
        let result = lm.think(&nats, "twenty-questions").await;
        assert!(result.is_err());
    }

    // ── ScriptedBackend helpers ───────────────────────────────────────────────

    #[test]
    fn scripted_silent_has_end_turn_stop_reason() {
        assert_eq!(ScriptedBackend::silent().stop_reason, "end_turn");
    }

    #[test]
    fn scripted_speak_has_tool_use_stop_reason_and_correct_input() {
        let r = ScriptedBackend::speak("hello");
        assert_eq!(r.stop_reason, "tool_use");
        assert_eq!(r.content[0].name.as_deref(), Some("speak"));
        assert_eq!(r.content[0].input.as_ref().unwrap()["message"], "hello");
    }

    #[test]
    fn scripted_leave_has_leave_game_tool() {
        let r = ScriptedBackend::leave();
        assert_eq!(r.content[0].name.as_deref(), Some("leave_game"));
    }
}

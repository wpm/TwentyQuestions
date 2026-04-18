use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

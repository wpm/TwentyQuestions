use free_agent::{Control, Entity, Envelope, OnIdle};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::language_model::{LanguageModel, ThinkResult};
use crate::message::Message;

const IDLE_TIMEOUT: Duration = Duration::from_secs(3);

/// Sentinel sender value used by the idle timer to trigger a think cycle
/// without a real incoming message.
const IDLE_SENTINEL: &str = "";

pub struct GameSession {
    handles: Vec<JoinHandle<()>>,
}

impl GameSession {
    pub async fn start(
        nats_url: &str,
        topic: &str,
        player_count: u32,
        model: &str,
        object: &str,
        api_key: &str,
    ) -> Result<Self, String> {
        let nats = async_nats::connect(nats_url)
            .await
            .map_err(|e| e.to_string())?;

        let mut handles = Vec::new();

        // Spawn one entity per participant (host + players), all on the shared topic.
        let language_models: Vec<Arc<Mutex<LanguageModel>>> = {
            let mut lms = vec![Arc::new(Mutex::new(LanguageModel::new_host(
                api_key, model, object, topic,
            )))];
            for index in 1..=player_count {
                lms.push(Arc::new(Mutex::new(LanguageModel::new_player(
                    api_key, model, index, topic,
                ))));
            }
            lms
        };

        for language_model in language_models {
            let subject = topic.to_string();

            // Idle timer: enqueue a sentinel message after IDLE_TIMEOUT of silence.
            let idle_timer: OnIdle<Message> = (
                IDLE_TIMEOUT,
                Box::new(|tx| {
                    let _ = tx.send(Message::new(IDLE_SENTINEL, ""));
                }),
            );

            let entity = Entity::<Message>::new(topic, Some(idle_timer));
            let entity_nats = nats.clone();

            handles.push(tokio::spawn(async move {
                entity
                    .run(entity_nats.clone(), move |msg: Message, nats| {
                        let language_model = language_model.clone();
                        let subject = subject.clone();
                        async move {
                            let mut lm = language_model.lock().await;

                            // Idle sentinel: think without recording a message.
                            // Real message: record it (skip if own), then think.
                            if msg.sender == IDLE_SENTINEL {
                                tracing::debug!("{}: idle timeout", lm.name());
                            } else if !lm.record_message(&msg) {
                                return; // own message — ignore
                            }

                            match lm.think(&nats, &subject).await {
                                Ok(ThinkResult::Silent) => {}
                                Ok(ThinkResult::Left) => {
                                    tracing::info!("{} left the game", lm.name())
                                }
                                Err(e) => tracing::warn!("{} language model error: {e}", lm.name()),
                            }
                        }
                    })
                    .await;
            }));
        }

        publish_control(&nats, topic, Control::Start).await?;

        Ok(Self { handles })
    }

    pub async fn stop(&mut self, nats_url: &str, topic: &str) -> Result<(), String> {
        let nats = async_nats::connect(nats_url)
            .await
            .map_err(|e| e.to_string())?;
        publish_control(&nats, topic, Control::Stop).await?;
        for handle in self.handles.drain(..) {
            let _ = handle.await;
        }
        Ok(())
    }
}

async fn publish_control(
    nats: &async_nats::Client,
    subject: &str,
    ctrl: Control,
) -> Result<(), String> {
    let payload =
        serde_json::to_vec(&Envelope::<Message>::Control(ctrl)).map_err(|e| e.to_string())?;
    nats.publish(subject.to_string(), payload.into())
        .await
        .map_err(|e| e.to_string())?;
    nats.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

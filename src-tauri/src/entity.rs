use free_agent::{Control, Entity, Envelope};
use serde_json::Value;
use tokio::task::JoinHandle;

type Msg = Value;

pub struct GameSession {
    handles: Vec<JoinHandle<()>>,
}

impl GameSession {
    pub async fn start(nats_url: &str, topic: &str, player_count: u32) -> Result<Self, String> {
        let nats = async_nats::connect(nats_url)
            .await
            .map_err(|e| e.to_string())?;

        let mut handles = Vec::new();

        // Host has two entity tasks: one on the topic, one on topic.host
        for subject in [topic.to_string(), format!("{}.host", topic)] {
            let entity = Entity::<Msg>::new(subject, None);
            let entity_nats = nats.clone();
            handles.push(tokio::spawn(async move {
                entity.run(entity_nats, |_msg, _nats| async {}).await;
            }));
        }

        // Each player subscribes to the topic
        for _ in 1..=player_count {
            let entity = Entity::<Msg>::new(topic, None);
            let entity_nats = nats.clone();
            handles.push(tokio::spawn(async move {
                entity.run(entity_nats, |_msg, _nats| async {}).await;
            }));
        }

        publish_control(&nats, topic, Control::Start).await?;
        publish_control(&nats, &format!("{}.host", topic), Control::Start).await?;

        Ok(Self { handles })
    }

    pub async fn stop(&mut self, nats_url: &str, topic: &str) -> Result<(), String> {
        let nats = async_nats::connect(nats_url)
            .await
            .map_err(|e| e.to_string())?;
        publish_control(&nats, topic, Control::Stop).await?;
        publish_control(&nats, &format!("{}.host", topic), Control::Stop).await?;
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
    let payload = serde_json::to_vec(&Envelope::<Msg>::Control(ctrl)).map_err(|e| e.to_string())?;
    nats.publish(subject.to_string(), payload.into())
        .await
        .map_err(|e| e.to_string())?;
    nats.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

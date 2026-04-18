use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartGameArgs<'a> {
    nats_url: &'a str,
    topic: &'a str,
    player_count: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopGameArgs<'a> {
    nats_url: &'a str,
    topic: &'a str,
}

pub async fn invoke_start_game(
    nats_url: &str,
    topic: &str,
    player_count: u32,
) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&StartGameArgs {
        nats_url,
        topic,
        player_count,
    })
    .map_err(|e| e.to_string())?;
    invoke("start_game", args).await;
    Ok(())
}

pub async fn invoke_stop_game(nats_url: &str, topic: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&StopGameArgs { nats_url, topic })
        .map_err(|e| e.to_string())?;
    invoke("stop_game", args).await;
    Ok(())
}

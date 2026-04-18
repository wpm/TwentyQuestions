use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize)]
struct GetArgs {
    key: String,
}

#[derive(Serialize)]
struct SetArgs {
    key: String,
    value: String,
}

pub async fn get_setting(key: &str) -> Option<String> {
    let args = serde_wasm_bindgen::to_value(&GetArgs {
        key: key.to_string(),
    })
    .ok()?;
    invoke("get_setting", args).await.as_string()
}

pub async fn set_setting(key: &str, value: &str) {
    if let Ok(args) = serde_wasm_bindgen::to_value(&SetArgs {
        key: key.to_string(),
        value: value.to_string(),
    }) {
        invoke("set_setting", args).await;
    }
}

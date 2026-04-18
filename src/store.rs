use serde::Serialize;
use wasm_bindgen::prelude::*;

pub const STORE_PATH: &str = "settings.json";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize)]
struct LoadArgs<'a> {
    path: &'a str,
}

#[derive(Serialize)]
struct GetArgs<'a> {
    rid: u32,
    key: &'a str,
}

#[derive(Serialize)]
struct SaveArgs {
    rid: u32,
}

pub async fn load_store(path: &str) -> Option<u32> {
    let args = serde_wasm_bindgen::to_value(&LoadArgs { path }).ok()?;
    let rid = invoke("plugin:store|load", args).await;
    rid.as_f64().map(|n| n as u32)
}

pub async fn store_get_string(rid: u32, key: &str) -> Option<String> {
    let args = serde_wasm_bindgen::to_value(&GetArgs { rid, key }).ok()?;
    let result = invoke("plugin:store|get", args).await;
    let arr = js_sys::Array::from(&result);
    let exists = arr.get(1).as_bool().unwrap_or(false);
    if exists {
        arr.get(0).as_string()
    } else {
        None
    }
}

pub async fn store_set_string(rid: u32, key: &str, value: &str) {
    // Build args object manually since JsValue doesn't implement Serialize
    let args = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &args,
        &JsValue::from_str("rid"),
        &JsValue::from_f64(rid as f64),
    );
    let _ = js_sys::Reflect::set(&args, &JsValue::from_str("key"), &JsValue::from_str(key));
    let _ = js_sys::Reflect::set(
        &args,
        &JsValue::from_str("value"),
        &JsValue::from_str(value),
    );
    invoke("plugin:store|set", args.into()).await;

    if let Ok(save_args) = serde_wasm_bindgen::to_value(&SaveArgs { rid }) {
        invoke("plugin:store|save", save_args).await;
    }
}

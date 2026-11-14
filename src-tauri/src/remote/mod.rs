mod client;
#[path = "../../../sidecar-rs/src/mobile/wire.rs"]
mod wire;
pub use client::{connect, pair, Identity, Session};

#[allow(dead_code)]
#[path = "../../../sidecar-rs/src/mobile/desktop_routes.rs"]
mod desktop_routes;
mod proxy;
#[path = "../../../sidecar-rs/src/mobile/tunnel.rs"]
mod tunnel;
pub use proxy::{start_proxy, Proxy};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use tauri::Emitter;

struct RemoteState {
    proxy: Proxy,
    events: tokio::task::JoinHandle<()>,
    status: std::sync::Arc<std::sync::Mutex<String>>,
    host_name: String,
}
impl Drop for RemoteState {
    fn drop(&mut self) {
        self.events.abort();
    }
}
static REMOTE: std::sync::Mutex<Option<RemoteState>> = std::sync::Mutex::new(None);
static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
fn active_path() -> Result<std::path::PathBuf, String> {
    Ok(crate::store::base_dir()?.join("remote-active"))
}
#[tauri::command]
pub fn local_sidecar_endpoint() -> String {
    format!("127.0.0.1:{}", crate::sidecar_port())
}
#[tauri::command]
pub fn sidecar_endpoint() -> String {
    REMOTE
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.proxy.endpoint.clone())
        .unwrap_or_else(local_sidecar_endpoint)
}
#[tauri::command]
pub fn remote_status() -> Value {
    let state = REMOTE.lock().unwrap();
    match state.as_ref() {
        Some(s) => {
            json!({"status": *s.status.lock().unwrap(), "hostName": s.host_name, "remembered": true})
        }
        None => {
            let identity = Identity::load().ok();
            json!({"status":"disconnected", "hostName":identity.as_ref().map(|i| &i.host_name), "remembered":identity.is_some()})
        }
    }
}
#[tauri::command]
pub async fn remote_pair(
    app: tauri::AppHandle,
    uri: String,
    name: String,
) -> Result<String, String> {
    pair(&uri, &name).await?;
    remote_connect(app).await
}
async fn activate(app: tauri::AppHandle, offline: bool) -> Result<String, String> {
    let identity = Identity::load()?;
    let proxy = if offline {
        proxy::start_offline(&identity).await?
    } else {
        start_proxy(&identity).await?
    };
    let mut events = proxy.subscribe();
    let status = std::sync::Arc::new(std::sync::Mutex::new(
        if proxy.connected() {
            "connected"
        } else {
            "reconnecting"
        }
        .to_owned(),
    ));
    let event_status = status.clone();
    let event_app = app.clone();
    let task = tokio::spawn(async move {
        loop {
            let event = match events.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    json!({"event":"resync"})
                }
                Err(_) => break,
            };
            if let Some(name) = event["event"].as_str() {
                if name == "connected" || name == "reconnecting" {
                    *event_status.lock().unwrap() = name.to_owned();
                    let _ = event_app.emit("remote:status", remote_status());
                } else if [
                    "git:repo",
                    "diff:changed",
                    "note:changed",
                    "store:changed",
                    "file:changed",
                    "run:manifests",
                    "resync",
                ]
                .contains(&name)
                {
                    let _ = event_app.emit(name, event["payload"].clone());
                }
            }
        }
    });
    let endpoint = proxy.endpoint.clone();
    *REMOTE.lock().unwrap() = Some(RemoteState {
        proxy,
        events: task,
        status,
        host_name: identity.host_name,
    });
    if let Ok(path) = active_path() {
        let _ = std::fs::write(path, b"1");
    }
    let _ = app.emit("sidecar:endpoint", &endpoint);
    Ok(endpoint)
}
#[tauri::command]
pub async fn remote_connect(app: tauri::AppHandle) -> Result<String, String> {
    let _guard = INIT.lock().await;
    activate(app, false).await
}
#[tauri::command]
pub async fn remote_bootstrap(app: tauri::AppHandle) -> Result<(), String> {
    let _guard = INIT.lock().await;
    let inactive = REMOTE.lock().unwrap().is_none();
    if inactive && active_path()?.exists() && Identity::load().is_ok() {
        activate(app, true).await?;
    }
    Ok(())
}
#[tauri::command]
pub async fn remote_invoke(command: String, args: Value) -> Result<Value, String> {
    let caller = REMOTE
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.proxy.caller())
        .ok_or("Remote host disconnected")?;
    let mut result = caller.call(&command, args).await?;
    if ["read_file_bytes", "git_blob", "read_temp_image"].contains(&command.as_str()) {
        let mut bytes = Vec::new();
        loop {
            bytes.extend(
                STANDARD
                    .decode(result["bytes"].as_str().ok_or("Invalid byte chunk")?)
                    .map_err(|e| e.to_string())?,
            );
            match result["next"].as_str() {
                Some(token) => {
                    result = caller
                        .call("read_bytes_chunk", json!({"token": token}))
                        .await?
                }
                None => return serde_json::to_value(bytes).map_err(|e| e.to_string()),
            }
        }
    }
    Ok(result)
}
#[tauri::command]
pub fn remote_disconnect(app: tauri::AppHandle) {
    *REMOTE.lock().unwrap() = None;
    if let Ok(path) = active_path() {
        let _ = std::fs::remove_file(path);
    }
    let _ = app.emit("sidecar:endpoint", local_sidecar_endpoint());
}
#[tauri::command]
pub fn remote_forget(app: tauri::AppHandle) -> Result<(), String> {
    remote_disconnect(app);
    Identity::forget()
}

use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::Listener;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

fn secret_path() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("PANORAMA_CONFIG_DIR") { return Some(std::path::PathBuf::from(dir).join("host-service-secret")); }
    let name = if cfg!(debug_assertions) { ".panorama-dev" } else { ".panorama" };
    Some(dirs::home_dir()?.join(name).join("host-service-secret"))
}
fn arg<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null)).map_err(|e| format!("Invalid {key}: {e}"))
}
fn value<T: serde::Serialize>(result: T) -> Result<Value, String> { serde_json::to_value(result).map_err(|e| e.to_string()) }

fn bytes(response: tauri::ipc::Response) -> Result<Vec<u8>, String> {
    use tauri::ipc::{IpcResponse, InvokeResponseBody};
    match response.body().map_err(|e| e.to_string())? {
        InvokeResponseBody::Raw(bytes) => Ok(bytes),
        _ => Err("Expected bytes".into()),
    }
}

pub async fn dispatch(command: &str, args: &Value) -> Result<Value, String> {
    match command {
        "store_read" => value(crate::store::store_read(arg(args, "name")?)?),
        "store_write" => value(crate::store::store_write(arg(args, "name")?, arg(args, "value")?)?),
        "store_write_many" => value(crate::store::store_write_many(arg(args, "entries")?)?),
        "store_delete" => value(crate::store::store_delete(arg(args, "name")?)?),
        "store_list" => value(crate::store::store_list(arg(args, "dir")?)?),
        "read_dir" => value(crate::read_dir(arg(args, "path")?).await?),
        "read_file_bytes" => value(bytes(crate::read_file_bytes(arg(args, "path")?)?)?),
        "read_text_file" => value(crate::read_text_file(arg(args, "path")?)?),
        "write_text_file" => value(crate::write_text_file(arg(args, "path")?, arg(args, "content")?)?),
        "create_entry" => value(crate::create_entry(arg(args, "path")?, arg(args, "dir")?)?),
        "git_repos" => value(crate::git::git_repos(arg(args, "path")?).await?),
        "git_status" => value(crate::git::git_status(arg(args, "path")?, arg(args, "limit")?).await?),
        "git_commit" => value(crate::git::git_commit(arg(args, "path")?, arg(args, "files")?, arg(args, "message")?, arg(args, "amend")?).await?),
        "git_log_messages" => value(crate::git::git_log_messages(arg(args, "path")?, arg(args, "limit")?).await?),
        "git_log_graph" => value(crate::git::git_log_graph(arg(args, "path")?, arg(args, "limit")?).await?),
        "git_remote_url" => value(crate::git::git_remote_url(arg(args, "path")?).await?),
        "git_unpushed_commits" => value(crate::git::git_unpushed_commits(arg(args, "path")?).await?),
        "git_branches" => value(crate::git::git_branches(arg(args, "path")?).await?),
        "git_checkout" => value(crate::git::git_checkout(arg(args, "path")?, arg(args, "branch")?, arg(args, "stash")?).await?),
        "git_fetch" => value(crate::git::git_fetch(arg(args, "path")?).await?),
        "git_create_branch" => value(crate::git::git_create_branch(arg(args, "path")?, arg(args, "name")?, arg(args, "checkout")?, arg(args, "overwrite")?, arg(args, "startPoint")?).await?),
        "git_rename_branch" => value(crate::git::git_rename_branch(arg(args, "path")?, arg(args, "oldName")?, arg(args, "newName")?).await?),
        "git_delete_branch" => value(crate::git::git_delete_branch(arg(args, "path")?, arg(args, "fullName")?, arg(args, "isRemote")?).await?),
        "git_resolve_ref" => value(crate::git::git_resolve_ref(arg(args, "path")?, arg(args, "reference")?).await?),
        "git_merge_branch" => value(crate::git::git_merge_branch(arg(args, "path")?, arg(args, "branch")?).await?),
        "git_rebase_onto" => value(crate::git::git_rebase_onto(arg(args, "path")?, arg(args, "branch")?).await?),
        "git_update_branch" => value(crate::git::git_update_branch(arg(args, "path")?, arg(args, "rebase")?).await?),
        "git_push_current" => value(crate::git::git_push_current(arg(args, "path")?).await?),
        "git_set_upstream" => value(crate::git::git_set_upstream(arg(args, "path")?, arg(args, "branch")?, arg(args, "upstream")?).await?),
        "git_compare_with_current" => value(crate::git::git_compare_with_current(arg(args, "path")?, arg(args, "branch")?).await?),
        "git_toggle_branch_favorite" => value(crate::git::git_toggle_branch_favorite(arg(args, "path")?, arg(args, "fullName")?).await?),
        "git_blob" => value(bytes(crate::git::git_blob(arg(args, "path")?, arg(args, "file")?, arg(args, "old")?, arg(args, "commit")?, arg(args, "base")?).await?)?),
        "git_commit_files" => value(crate::git::git_commit_files(arg(args, "path")?, arg(args, "commit")?).await?),
        "git_diff_with_ref" => value(crate::git::git_diff_with_ref(arg(args, "path")?, arg(args, "reference")?).await?),
        "git_commit_diff_file" => value(crate::git::git_commit_diff_file(arg(args, "path")?, arg(args, "commit")?, arg(args, "file")?).await?),
        "git_diff_file" => value(crate::git::git_diff_file(arg(args, "path")?, arg(args, "file")?, arg(args, "base")?).await?),
        "git_operation" => value(crate::git::git_operation(arg(args, "path")?, arg(args, "action")?).await?),
        "git_rollback_file" => value(crate::git::git_rollback_file(arg(args, "path")?, arg(args, "file")?).await?),
        "git_add_ignore" => value(crate::git::git_add_ignore(arg(args, "path")?, arg(args, "pattern")?, arg(args, "local")?).await?),
        "git_revert_hunk" => value(crate::git::git_revert_hunk(arg(args, "path")?, arg(args, "file")?, arg(args, "content")?, arg(args, "crlf")?).await?),
        "read_note" => value(crate::notes::read_note(arg(args, "wsId")?, arg(args, "noteId")?)),
        "write_note" => value(crate::notes::write_note(arg(args, "wsId")?, arg(args, "noteId")?, arg(args, "content")?)?),
        "delete_note" => value(crate::notes::delete_note(arg(args, "wsId")?, arg(args, "noteId")?)?),
        "link_note" => value(crate::notes::link_note(arg(args, "wsId")?, arg(args, "noteId")?, arg(args, "termTileId")?, arg(args, "title")?)?),
        "link_term" => value(crate::notes::link_term(arg(args, "aId")?, arg(args, "aName")?, arg(args, "bId")?, arg(args, "bName")?)?),
        "unlink_term" => value(crate::notes::unlink_term(arg(args, "aId")?, arg(args, "bId")?)?),
        "unlink_note" => value(crate::notes::unlink_note(arg(args, "noteId")?, arg(args, "termTileId")?)?),
        "docker_available" => value(crate::docker::docker_available().await),
        "docker_ps" => value(crate::docker::docker_ps().await?),
        "docker_engine" => value(crate::docker::docker_engine(arg(args, "action")?).await?),
        "docker_action" => value(crate::docker::docker_action(arg(args, "id")?, arg(args, "action")?).await?),
        "agent_usage" => value(crate::usage::agent_usage(arg(args, "provider")?).await?),
        "agent_history" => value(crate::history::agent_history(arg(args, "agent")?, arg(args, "cwd")?, arg(args, "tileId")?).await?),
        "pi_session_summary" => value(crate::pi::pi_session_summary(arg(args, "sessionId")?)),
        "claude_session_summary" => value(crate::claude::claude_session_summary(arg(args, "sessionId")?, arg(args, "cwd")?)),
        _ => Err("Command not allowed".into()),
    }
}

#[derive(Default)]
pub struct Watches { git_files: Vec<u32>, repos: Vec<String>, files: Vec<u32>, runs: Vec<u32>, chunks: std::collections::HashMap<String, (Vec<u8>, usize)>, uploads: std::collections::HashMap<String, Vec<u8>>, next: u64 }
impl Watches {
    async fn dispatch(&mut self, app: &tauri::AppHandle, command: &str, args: &Value) -> Option<Result<Value, String>> {
        if let Some(result) = self.data(command, args).await { return Some(result); }
        Some(match command {
            "git_watch_file" => match (|| Ok((arg(args, "path")?, arg(args, "file")?)))() {
                Ok((path, file)) => match crate::git::git_watch_file(app.clone(), path, file).await { Ok(id) => { self.git_files.push(id); value(id) }, Err(e) => Err(e) }, Err(e) => Err(e),
            },
            "git_watch_repo" => match arg(args, "path") {
                Ok(path) => match crate::git::git_watch_repo(app.clone(), path).await {
                    Ok(repo) => match value(repo) { Ok(v) => { if let Some(root) = v["root"].as_str() { self.repos.push(root.to_owned()); } Ok(v) }, Err(e) => Err(e) }, Err(e) => Err(e),
                }, Err(e) => Err(e),
            },
            "watch_text_file" => match arg(args, "path").and_then(|p| crate::watch_text_file(app.clone(), p)) { Ok(id) => { self.files.push(id); value(id) }, Err(e) => Err(e) },
            "run_watch_manifests" => match arg(args, "path").and_then(|p| crate::run_watch_manifests(app.clone(), p)) { Ok(id) => { self.runs.push(id); value(id) }, Err(e) => Err(e) },
            "git_unwatch_file" => match arg(args, "id") { Ok(id) if take(&mut self.git_files, &id) => { crate::git::git_unwatch_file(id).await; Ok(Value::Null) }, _ => Err("Unknown watch".into()) },
            "git_unwatch_repo" => match arg(args, "root") { Ok(root) if take(&mut self.repos, &root) => { crate::git::git_unwatch_repo(root).await; Ok(Value::Null) }, _ => Err("Unknown watch".into()) },
            "unwatch_text_file" => match arg(args, "id") { Ok(id) if take(&mut self.files, &id) => { crate::unwatch_text_file(id); Ok(Value::Null) }, _ => Err("Unknown watch".into()) },
            "run_unwatch_manifests" => match arg(args, "id") { Ok(id) if take(&mut self.runs, &id) => { crate::run_unwatch_manifests(id); Ok(Value::Null) }, _ => Err("Unknown watch".into()) },
            _ => return None,
        })
    }
    pub async fn data(&mut self, command: &str, args: &Value) -> Option<Result<Value, String>> {
        Some(match command {
            "read_file_bytes" | "git_blob" | "read_temp_image" => {
                let result = match command {
                    "read_file_bytes" => arg(args, "path").and_then(crate::read_file_bytes).and_then(bytes),
                    "read_temp_image" => arg(args, "path").and_then(crate::read_temp_image).and_then(bytes),
                    _ => match (|| Ok((arg(args, "path")?, arg(args, "file")?, arg(args, "old")?, arg(args, "commit")?, arg(args, "base")?)))() {
                        Ok((path, file, old, commit, base)) => crate::git::git_blob(path, file, old, commit, base).await.and_then(bytes), Err(e) => Err(e),
                    }
                };
                match result { Ok(data) => { self.next += 1; let token = self.next.to_string(); self.chunks.insert(token.clone(), (data, 0)); Ok(self.chunk(&token)) }, Err(e) => Err(e) }
            },
            "read_bytes_chunk" => match arg::<String>(args, "token") { Ok(token) if self.chunks.contains_key(&token) => Ok(self.chunk(&token)), _ => Err("Unknown byte stream".into()) },
            "upload_image_chunk" => (|| {
                let name: String = arg(args, "name")?;
                let data: Vec<u8> = arg(args, "bytes")?;
                let first: bool = arg(args, "first")?;
                let last: bool = arg(args, "last")?;
                if data.len() > 32768 { return Err("Image chunk too large".into()); }
                if first { if self.uploads.len() >= 8 && !self.uploads.contains_key(&name) { return Err("Too many uploads".into()); } self.uploads.insert(name.clone(), Vec::new()); }
                let upload = self.uploads.get_mut(&name).ok_or("Unknown upload")?;
                if upload.len() + data.len() > 5 * 1024 * 1024 { self.uploads.remove(&name); return Err("Image too large".into()); }
                upload.extend(data);
                if last { let data = self.uploads.remove(&name).unwrap(); value(crate::save_temp_image(&name, &data)?) } else { Ok(Value::Null) }
            })(),
            _ => return None,
        })
    }
    fn chunk(&mut self, token: &str) -> Value {
        let (data, offset) = self.chunks.get_mut(token).unwrap();
        let end = (*offset + 256 * 1024).min(data.len());
        let encoded = STANDARD.encode(&data[*offset..end]);
        *offset = end;
        let more = end < data.len();
        if !more { self.chunks.remove(token); }
        json!({"bytes": encoded, "next": if more { Some(token) } else { None }})
    }
    async fn cleanup(self) {
        for id in self.git_files { crate::git::git_unwatch_file(id).await; }
        for root in self.repos { crate::git::git_unwatch_repo(root).await; }
        for id in self.files { crate::unwatch_text_file(id); }
        for id in self.runs { crate::run_unwatch_manifests(id); }
    }
}
fn take<T: PartialEq>(list: &mut Vec<T>, item: &T) -> bool {
    if let Some(index) = list.iter().position(|v| v == item) { list.remove(index); true } else { false }
}

pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(path) = secret_path() {
                if let Ok(secret) = std::fs::read_to_string(path) {
                    let url = format!("ws://127.0.0.1:{}/host-service", crate::sidecar_port());
                    if let Ok(mut request) = url.into_client_request() {
                        if let Ok(auth) = format!("Bearer {}", secret.trim()).parse() {
                            request.headers_mut().insert("authorization", auth);
                            if let Ok((ws, _)) = tokio_tungstenite::connect_async_with_config(request, None, true).await {
                                let (mut sink, mut stream) = ws.split();
                                let (tx, mut rx) = tokio::sync::mpsc::channel::<Value>(64);
                                let mut listeners = Vec::new();
                                let mut watches = Watches::default();
                                let mut replaced = false;
                                for event in ["git:repo", "diff:changed", "note:changed", "store:changed", "file:changed", "run:manifests"] {
                                    let tx = tx.clone();
                                    listeners.push(app.listen(event, move |e| { let payload = serde_json::from_str::<Value>(e.payload()).unwrap_or(Value::Null); let _ = tx.try_send(json!({"event": event, "payload": payload})); }));
                                }
                                loop {
                                    tokio::select! {
                                        incoming = stream.next() => match incoming {
                                            Some(Ok(Message::Text(s))) => if let Ok(v) = serde_json::from_str::<Value>(&s) {
                                                if let (Some(id), Some(cmd)) = (v.get("id"), v.get("command").and_then(Value::as_str)) {
                                                    let result = match watches.dispatch(&app, cmd, &v["args"]).await { Some(r) => r, None => dispatch(cmd, &v["args"]).await };
                                                    let response = match result { Ok(r) => json!({"id": id, "result": r}), Err(e) => json!({"id": id, "error": e}) };
                                                    if sink.send(Message::Text(response.to_string().into())).await.is_err() { break; }
                                                }
                                            },
                                            Some(Ok(Message::Close(frame))) => { replaced = frame.as_ref().is_some_and(|f| f.reason == "provider replaced"); break; },
                                            Some(Err(_)) | None => break,
                                            _ => {},
                                        },
                                        event = rx.recv() => match event { Some(v) => if sink.send(Message::Text(v.to_string().into())).await.is_err() { break; }, None => break },
                                    }
                                }
                                for id in listeners { app.unlisten(id); }
                                watches.cleanup().await;
                                if replaced { return; }
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_ownership_removes_one_registration() {
        let mut watches = vec![1, 1, 2];
        assert!(take(&mut watches, &1));
        assert_eq!(watches, vec![1, 2]);
        assert!(!take(&mut watches, &3));
    }
    #[tokio::test]
    async fn chunked_bytes_and_image_upload() {
        let mut watches = Watches::default();
        let name = format!("test-paste-{}.png", std::process::id());
        let data = vec![42u8; 270000];
        let mut path = Value::Null;
        let count = data.chunks(32768).len();
        for (index, chunk) in data.chunks(32768).enumerate() {
            path = watches.data("upload_image_chunk", &json!({"name":name,"bytes":chunk,"first":index == 0,"last":index + 1 == count})).await.unwrap().unwrap();
        }
        let path = path.as_str().unwrap();
        let first = watches.data("read_temp_image", &json!({"path":path})).await.unwrap().unwrap();
        assert_eq!(STANDARD.decode(first["bytes"].as_str().unwrap()).unwrap(), data[..256 * 1024]);
        let token = first["next"].as_str().unwrap();
        let last = watches.data("read_bytes_chunk", &json!({"token":token})).await.unwrap().unwrap();
        assert_eq!(STANDARD.decode(last["bytes"].as_str().unwrap()).unwrap(), data[256 * 1024..]);
        assert!(last["next"].is_null());
        assert!(watches.data("read_bytes_chunk", &json!({"token":token})).await.unwrap().is_err());
        assert!(watches.data("upload_image_chunk", &json!({"name":"../bad", "bytes":[], "first":true, "last":true})).await.unwrap().is_err());
        assert!(watches.data("read_temp_image", &json!({"path":format!("{path}/../other")})).await.unwrap().is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn empty_byte_stream_finishes_immediately() {
        let mut watches = Watches::default();
        watches.chunks.insert("empty".into(), (Vec::new(), 0));
        assert_eq!(watches.chunk("empty"), json!({"bytes":"", "next":null}));
        assert!(watches.chunks.is_empty());
    }
    #[tokio::test]
    async fn real_git_and_bytes() {
        let dir = std::env::temp_dir().join(format!("panorama-bridge-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(std::process::Command::new("git").args(["init", "-q"]).arg(&dir).status().unwrap().success());
        let path = dir.to_string_lossy().to_string();
        assert!(dispatch("git_status", &json!({"path": path})).await.is_ok());
        let file = dir.join("binary");
        std::fs::write(&file, [0, 255, 42]).unwrap();
        assert_eq!(dispatch("read_file_bytes", &json!({"path": file})).await.unwrap(), json!([0, 255, 42]));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    async fn rejects_unlisted_and_validates_args() {
        assert_eq!(dispatch("open_url", &json!({})).await.unwrap_err(), "Command not allowed");
        assert!(dispatch("store_read", &json!({})).await.is_err());
    }
}

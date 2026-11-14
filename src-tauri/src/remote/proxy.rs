use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::mpsc};
use tokio_tungstenite::tungstenite::Message;
use super::{client, desktop_routes, tunnel, Identity};

struct Open { id: u64, method: String, target: String, ws: bool, pipe: tokio::io::DuplexStream }
pub struct Proxy { pub endpoint: String, task: tokio::task::JoinHandle<()>, accept_task: tokio::task::AbortHandle }
impl Drop for Proxy { fn drop(&mut self) { self.task.abort(); self.accept_task.abort(); } }
const ORIGINS: &[&str] = &["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost", "http://localhost:1471", "http://127.0.0.1:1471"];
fn parse_request(head: &str, token: &str, port: u16) -> Option<(String, String, bool, String, Option<String>)> {
    let mut lines = head.split("\r\n"); let mut first = lines.next()?.split_whitespace();
    let method = first.next()?; let path = first.next()?; if first.next()? != "HTTP/1.1" || first.next().is_some() { return None; }
    let prefix = format!("/{token}"); let target = path.strip_prefix(&prefix)?;
    if !target.starts_with('/') { return None; }
    let mut headers = HashMap::new();
    for line in lines { if line.is_empty() { break; } let (name,value) = line.split_once(':')?; if headers.insert(name.to_ascii_lowercase(), value.trim()).is_some() { return None; } }
    if headers.get("host").copied() != Some(format!("127.0.0.1:{port}").as_str()) { return None; }
    if headers.get("origin").is_some_and(|o| !ORIGINS.contains(o)) { return None; }
    let ws = headers.get("upgrade").is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    if !desktop_routes::allowed(method, target, ws) { return None; }
    let tail = head.split_once("\r\n")?.1;
    Some((method.to_owned(), target.to_owned(), ws, format!("{method} {target} HTTP/1.1\r\n{tail}"), headers.get("origin").map(|v| v.to_string())))
}
pub async fn start_proxy(identity: &Identity) -> Result<Proxy, String> {
    let (mut ws, mut channel, _) = client::handshake(identity, json!({"t":"auth"})).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut bytes = [0u8; 24]; getrandom::getrandom(&mut bytes).map_err(|e| e.to_string())?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let endpoint = format!("127.0.0.1:{port}/{token}");
    let (open_tx, mut open_rx) = mpsc::channel::<Open>(64);
    let (out, mut incoming) = mpsc::channel::<Vec<u8>>(64);
    let accept_task = tokio::spawn(async move {
        let ids = Arc::new(AtomicU64::new(1));
        while let Ok((socket, _)) = listener.accept().await {
            let tx = open_tx.clone(); let token = token.clone(); let id = ids.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move { handle(socket, tx, &token, port, id).await; });
        }
    });
    let accept_abort = accept_task.abort_handle();
    let task = tokio::spawn(async move {
        let mut mux = tunnel::Mux::new(out.clone());
        loop { tokio::select! {
            Some(open) = open_rx.recv() => {
                let request = json!({"t":"stream","op":"open","id":open.id,"method":open.method,"target":open.target,"ws":open.ws});
                let mut plain = vec![0]; plain.extend_from_slice(request.to_string().as_bytes());
                if ws.send(Message::Binary(channel.seal(&plain).into())).await.is_err() { break; }
                if !mux.insert(open.id, open.pipe) { break; }
            }
            Some(plain) = incoming.recv() => {
                if plain.first() == Some(&0) { if let Ok(v) = serde_json::from_slice::<Value>(&plain[1..]) { if v["op"] == "close" { if let Some(id) = v["id"].as_u64() { mux.remove(id); } } } }
                if ws.send(Message::Binary(channel.seal(&plain).into())).await.is_err() { break; }
            }
            message = ws.next() => match message {
                Some(Ok(Message::Binary(data))) => { let Some(plain) = channel.open(&data) else { break }; if !mux.handle(&plain) { break; } },
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}, _ => break,
            }
        }}
        accept_task.abort();
    });
    Ok(Proxy { endpoint, task, accept_task: accept_abort })
}
async fn handle(mut socket: TcpStream, tx: mpsc::Sender<Open>, token: &str, port: u16, id: u64) {
    let mut head = Vec::new(); let mut byte = [0];
    let read = async { while head.len() < 16384 && !head.ends_with(b"\r\n\r\n") { socket.read_exact(&mut byte).await?; head.push(byte[0]); } Ok::<_, std::io::Error>(()) };
    if tokio::time::timeout(std::time::Duration::from_secs(10), read).await.ok().and_then(Result::ok).is_none() { return; }
    let parsed = std::str::from_utf8(&head).ok().filter(|_| head.ends_with(b"\r\n\r\n")).and_then(|h| parse_request(h, token, port));
    let Some((method, target, ws, rewritten, origin)) = parsed else { let _ = socket.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await; return };
    let (pipe, mut worker) = tokio::io::duplex(64 * 1024);
    if tx.send(Open { id, method, target, ws, pipe }).await.is_err() { return; }
    if worker.write_all(rewritten.as_bytes()).await.is_err() { return; }
    if ws { let _ = tokio::io::copy_bidirectional(&mut socket, &mut worker).await; return; }
    let (mut sr, mut sw) = socket.into_split(); let (mut wr, mut ww) = tokio::io::split(worker);
    let request = async { let _ = tokio::io::copy(&mut sr, &mut ww).await; let _ = ww.shutdown().await; };
    let response = async {
        let mut head = Vec::new(); let mut byte = [0];
        while head.len() < 16384 && !head.ends_with(b"\r\n\r\n") { if wr.read_exact(&mut byte).await.is_err() { return; } head.push(byte[0]); }
        if !head.ends_with(b"\r\n\r\n") { return; }
        let Ok(text) = std::str::from_utf8(&head) else { return };
        let mut filtered = String::new();
        for line in text.split("\r\n").filter(|l| !l.is_empty()) {
            if line.split_once(':').is_some_and(|(name,_)| name.eq_ignore_ascii_case("access-control-allow-origin")) { continue; }
            filtered.push_str(line); filtered.push_str("\r\n");
        }
        if let Some(origin) = origin { filtered.push_str(&format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n")); }
        filtered.push_str("\r\n");
        if sw.write_all(filtered.as_bytes()).await.is_ok() { let _ = tokio::io::copy(&mut wr, &mut sw).await; }
        let _ = sw.shutdown().await;
    };
    tokio::join!(request, response);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn request_policy() {
        let head = "GET /abc/pty?tileId=x HTTP/1.1\r\nHost: 127.0.0.1:123\r\nOrigin: tauri://localhost\r\nUpgrade: websocket\r\n\r\n";
        assert!(parse_request(head,"abc",123).is_some());
        assert!(parse_request(head,"bad",123).is_none());
        assert!(parse_request(&head.replace("tauri://localhost", "https://evil.test"),"abc",123).is_none());
        assert!(parse_request(&head.replace("/pty?tileId=x", "/mobile/state"),"abc",123).is_none());
        assert!(parse_request(&head.replace("127.0.0.1:123", "evil.test"),"abc",123).is_none());
    }
}

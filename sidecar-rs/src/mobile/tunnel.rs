use std::{collections::HashMap, sync::Arc};
use serde_json::{json, Value};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, DuplexStream}, sync::{mpsc, Semaphore}};

const CHUNK: usize = 32 * 1024;
const WINDOW: usize = 4;
const MAX_STREAMS: usize = 64;
pub type Out = mpsc::Sender<Vec<u8>>;
struct Stream { input: mpsc::Sender<Option<Vec<u8>>>, credit: Arc<Semaphore>, tasks: Vec<tokio::task::AbortHandle> }
pub struct Mux { streams: HashMap<u64, Stream>, out: Out }
fn control(id: u64, op: &str) -> Vec<u8> { let mut v = vec![0]; v.extend_from_slice(json!({"t":"stream","id":id,"op":op}).to_string().as_bytes()); v }
#[allow(dead_code)]
pub fn send_control(out: &Out, value: Value) -> Result<(), ()> { let mut v = vec![0]; v.extend_from_slice(value.to_string().as_bytes()); out.try_send(v).map_err(|_| ()) }
impl Mux {
    pub fn new(out: Out) -> Self { Self { streams: HashMap::new(), out } }
    pub fn insert(&mut self, id: u64, pipe: DuplexStream) -> bool {
        if self.streams.len() >= MAX_STREAMS || self.streams.contains_key(&id) { return false; }
        let (input, mut incoming) = mpsc::channel::<Option<Vec<u8>>>(WINDOW + 1);
        let credit = Arc::new(Semaphore::new(WINDOW));
        let (mut read, mut write) = tokio::io::split(pipe);
        let out = self.out.clone();
        let reader_credit = credit.clone();
        let reader = tokio::spawn(async move {
            let credit = reader_credit;
            let mut buf = vec![0; CHUNK];
            loop {
                let Ok(permit) = credit.acquire().await else { break }; permit.forget();
                match read.read(&mut buf).await {
                    Ok(0) => { let _ = out.send(control(id, "eof")).await; break; }
                    Ok(n) => { let mut msg = Vec::with_capacity(9+n); msg.push(2); msg.extend_from_slice(&id.to_le_bytes()); msg.extend_from_slice(&buf[..n]); if out.send(msg).await.is_err() { break; } }
                    Err(_) => { let _ = out.send(control(id, "cancel")).await; break; }
                }
            }
        });
        let out = self.out.clone();
        let writer = tokio::spawn(async move {
            while let Some(data) = incoming.recv().await {
                match data {
                    Some(data) => { if write.write_all(&data).await.is_err() { let _ = out.send(control(id, "cancel")).await; return; } if out.send(control(id, "credit")).await.is_err() { return; } }
                    None => { let _ = write.shutdown().await; return; }
                }
            }
        });
        self.streams.insert(id, Stream { input, credit: credit.clone(), tasks: vec![reader.abort_handle(), writer.abort_handle()] });
        let out = self.out.clone();
        tokio::spawn(async move { let _ = tokio::join!(reader, writer); let _ = out.send(control(id, "close")).await; });
        true
    }
    pub fn handle(&mut self, plain: &[u8]) -> bool {
        if plain.first() == Some(&2) {
            if plain.len() < 10 || plain.len() > 9 + CHUNK { return false; }
            let id = u64::from_le_bytes(plain[1..9].try_into().unwrap());
            return self.streams.get(&id).is_some_and(|s| s.input.try_send(Some(plain[9..].to_vec())).is_ok());
        }
        let Ok(v) = serde_json::from_slice::<Value>(plain.get(1..).unwrap_or_default()) else { return false };
        if plain.first() != Some(&0) || v["t"] != "stream" { return false; }
        let Some(id) = v["id"].as_u64() else { return false };
        match v["op"].as_str() {
            Some("credit") => if let Some(s) = self.streams.get(&id) { if s.credit.available_permits() >= WINDOW { return false; } s.credit.add_permits(1); },
            Some("eof") => if let Some(s) = self.streams.get(&id) { if s.input.try_send(None).is_err() { return false; } },
            Some("cancel" | "error" | "close") => self.remove(id),
            _ => return false,
        }
        true
    }
    pub fn remove(&mut self, id: u64) { if let Some(s) = self.streams.remove(&id) { s.credit.close(); for task in s.tasks { task.abort(); } } }
}
impl Drop for Mux { fn drop(&mut self) { for (_, s) in self.streams.drain() { s.credit.close(); for task in s.tasks { task.abort(); } } } }

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn window_transfers_large_stream_and_rejects_oversize() {
        let (a_tx, mut a_rx) = mpsc::channel(64); let (b_tx, mut b_rx) = mpsc::channel(64);
        let mut a = Mux::new(a_tx); let mut b = Mux::new(b_tx);
        let (mut aw, ap) = tokio::io::duplex(4096); let (mut bw, bp) = tokio::io::duplex(4096);
        assert!(a.insert(7, ap)); assert!(b.insert(7, bp));
        assert!(!a.insert(7, tokio::io::duplex(1).0));
        let router = tokio::spawn(async move { loop { tokio::select! {
            Some(data) = a_rx.recv() => { if !b.handle(&data) { break; } },
            Some(data) = b_rx.recv() => { if !a.handle(&data) { break; } },
            else => break,
        }}});
        let data: Vec<u8> = (0..200*1024).map(|n| (n % 251) as u8).collect(); let expected = data.clone();
        let writer = tokio::spawn(async move { aw.write_all(&data).await.unwrap(); aw.shutdown().await.unwrap(); });
        let mut received = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), bw.read_to_end(&mut received)).await.unwrap().unwrap();
        assert_eq!(received, expected); writer.await.unwrap(); router.abort();
        let (out, _) = mpsc::channel(1); let mut mux = Mux::new(out);
        assert!(!mux.handle(&vec![2; CHUNK + 10]));
    }
}

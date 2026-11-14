#[path = "../../../sidecar-rs/src/mobile/wire.rs"]
mod wire;
mod client;
pub use client::{connect, pair, Identity, Session};

#[path = "../../../sidecar-rs/src/mobile/wire.rs"]
mod wire;
mod client;
pub use client::{connect, pair, Identity, Session};

#[path = "../../../sidecar-rs/src/mobile/tunnel.rs"]
mod tunnel;
#[allow(dead_code)]
#[path = "../../../sidecar-rs/src/mobile/desktop_routes.rs"]
mod desktop_routes;
mod proxy;
pub use proxy::{start_proxy, Proxy};

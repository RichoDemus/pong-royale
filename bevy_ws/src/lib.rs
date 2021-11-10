#[cfg(not(feature = "web"))]
pub use client::WebsocketPlugin;
#[cfg(not(feature = "web"))]
pub use client::WebsocketResource;
#[cfg(not(feature = "web"))]
pub use server::WebsocketServerPlugin;
#[cfg(not(feature = "web"))]
pub use server::WebsocketServerResource;
#[cfg(feature = "web")]
pub use ws_client::WebsocketPlugin;
#[cfg(feature = "web")]
pub use ws_client::WebsocketResource;

#[cfg(not(feature = "web"))]
pub mod client;
pub mod event_stream;
#[cfg(not(feature = "web"))]
pub mod server;
#[cfg(feature = "web")]
pub mod ws_client;

#[derive(Debug, Clone)]
pub enum WebsocketServerEvent {
    OnOpen(u64),
    OnMessage(u64, String),
    OnClose(u64),
}

#[derive(Debug, Clone)]
pub enum WebsocketClientEvent {
    OnOpen(u64),
    OnMessage(String),
    OnClose,
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use crate::server::WebsocketServerResource;

    #[test]
    fn test_once_cell() {
        let _res = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .try_init();
        let _resource = WebsocketServerResource::default();
    }
}

use std::sync::{Arc, Mutex};

use async_compat::Compat;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures::FutureExt;
use futures::SinkExt;
use futures_util::StreamExt;
use log::info;
use log::trace;
use log::warn;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::WebsocketClientEvent;

pub struct WebsocketPlugin;

struct MessageOffset(usize); // todo remove

impl Plugin for WebsocketPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WebsocketResource::default());
        app.add_event::<WebsocketClientEvent>();
        app.add_system(setup_websocket_system.system());
        app.insert_resource(MessageOffset(0));
        app.add_system(write_websocket_event_to_client.system());
    }
}

fn write_websocket_event_to_client(
    mut event_writer: EventWriter<WebsocketClientEvent>,
    resource: ResMut<WebsocketResource>,
) {
    let mut receiver = resource.ws_to_event_channel_receiver.lock().unwrap();
    match &mut *receiver {
        Some(receiver) => match receiver.try_recv() {
            Ok(msg) => {
                info!("system pusing event {:?} to client", msg);
                event_writer.send(msg);
            }
            Err(e) => trace!("error reading next event to be pushed {:?}", e),
        },
        None => trace!("no event waiting to be pushed"),
    }
    trace!("done with push system");
}

fn setup_websocket_system(mut resource: ResMut<WebsocketResource>, task_pool: Res<IoTaskPool>) {
    if let Some(address) = resource.address.take() {
        info!("Have an address to connect to: {address}");
        let (ws_to_event_channel_sender, ws_to_event_channel_receiver) =
            tokio::sync::mpsc::channel::<WebsocketClientEvent>(10);
        resource
            .ws_to_event_channel_receiver
            .lock()
            .unwrap()
            .replace(ws_to_event_channel_receiver);

        let (send_ws, mut message_to_send) = tokio::sync::mpsc::channel::<WebsocketClientEvent>(10);
        resource
            .message_to_be_sent_over_ws
            .lock()
            .unwrap()
            .replace(send_ws);

        let (shutdown_main_loop, listen_for_shutdown) = tokio::sync::oneshot::channel::<()>();
        resource
            .shutdown_main_loop
            .lock()
            .unwrap()
            .replace(shutdown_main_loop);

        let running = resource.run_listen_loop.clone();
        // create a separate task that will both listen to ws messages
        // and also take messages to send and send them
        task_pool.spawn(Compat::new(async move {
            let url = Url::parse(address.as_str()).unwrap();
            let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
            let (mut write, mut read) = ws_stream.split();

            let listen_for_shutdown = listen_for_shutdown.fuse();
            futures::pin_mut!(listen_for_shutdown);

            'main: while *running.lock().expect("lock") {
                trace!("loop");
                let next_incoming_message = read.next().fuse();
                let next_message_to_send = message_to_send.recv().fuse();
                futures::pin_mut!(next_incoming_message, next_message_to_send);
                futures::select! {
                    msg = next_incoming_message  => {
                        if let Some(result) = msg {
                            match result {
                                Ok(Message::Text(msg)) => {
                                    let str = msg.to_string();
                                    info!("Client received: {:?}", str);
                                    if str.contains("##CLIENT_ID##") {
                                        let mut split = str.split(" ");
                                        split.next();
                                        let id = split.next();
                                        let id = id.unwrap();
                                        ws_to_event_channel_sender.send(WebsocketClientEvent::OnOpen(id.parse().unwrap())).await.unwrap();
                                    } else {
                                        ws_to_event_channel_sender.send(WebsocketClientEvent::OnMessage(msg)).await.unwrap();
                                    }
                                },
                                Err(e) => log::info!("ws recv error: {:?}", e),
                        _ => {},
                            }
                        }
                    },
                    msg = next_message_to_send => {
                        if let Some(msg) = msg {
                            if let WebsocketClientEvent::OnMessage(str) = msg {
                                  write.send(Message::Text(str)).await
                                .unwrap_or_else(|e|warn!("Send failed:{:?}", e));
                            }

                        }
                    },
                    _shutdown = listen_for_shutdown => {
                        warn!("Got shutdown signal");
                        break 'main;
                    }
                }
            }
            warn!("ws poll stopping");
        })).detach();
    }
}

// use by the client to talk to server
pub struct WebsocketResource {
    address: Option<String>,
    pub ws_to_event_channel_receiver:
        Mutex<Option<tokio::sync::mpsc::Receiver<WebsocketClientEvent>>>,
    pub message_to_be_sent_over_ws: Mutex<Option<tokio::sync::mpsc::Sender<WebsocketClientEvent>>>,
    pub shutdown_main_loop: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    run_listen_loop: Arc<Mutex<bool>>,
}

impl Default for WebsocketResource {
    fn default() -> Self {
        Self {
            address: None,
            ws_to_event_channel_receiver: Default::default(),
            message_to_be_sent_over_ws: Default::default(),
            shutdown_main_loop: Default::default(),
            run_listen_loop: Arc::new(Mutex::new(true)),
        }
    }
}

impl WebsocketResource {
    pub fn open(&mut self, address: &str) {
        self.address = Some(address.to_string());
    }
    pub fn broadcast(&self, message: String) {
        let mut receiver = self.message_to_be_sent_over_ws.lock().unwrap();
        match &mut *receiver {
            Some(receiver) => {
                info!("Added {:?} to message_to_be_sent_over_ws", message);
                receiver
                    .blocking_send(WebsocketClientEvent::OnMessage(message))
                    .unwrap_or_else(|e| warn!("Broadcast failed:{:?}", e));
                return;
            }
            None => panic!("no sender"),
        }
    }
    pub fn close(&self) {
        *self.run_listen_loop.lock().unwrap() = false;
        self.message_to_be_sent_over_ws.lock().unwrap().take();
        self.ws_to_event_channel_receiver.lock().unwrap().take();

        let mut receiver = self.shutdown_main_loop.lock().unwrap();
        match receiver.take() {
            Some(receiver) => {
                info!("Sending shutdown signal");
                receiver
                    .send(())
                    .unwrap_or_else(|e| warn!("Shutdown signal failed:{:?}", e));
                return;
            }
            None => panic!("no sender"),
        }
    }
}

use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use async_compat::Compat;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures::task::Poll;
use futures::{SinkExt, StreamExt};
use log::info;
use log::trace;
use log::warn;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

use crate::event_stream::EventStream;
use crate::{WebsocketClientEvent, WebsocketServerEvent};

pub struct WebsocketServerPlugin;

impl Plugin for WebsocketServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WebsocketServerResource::default());
        app.add_event::<WebsocketServerEvent>();
        app.add_system(write_websocket_event_to_server.system());
        app.add_system(websocket_server_system.system());
    }
}

fn write_websocket_event_to_server(
    mut event_writer: EventWriter<WebsocketServerEvent>,
    server: ResMut<WebsocketServerResource>,
) {
    let mut receivers = server.ws_to_event_channel_receiver.lock().unwrap();
    let receivers = receivers.deref_mut();
    for receiver in receivers {
        match receiver.try_recv() {
            Ok(msg) => {
                info!("system pusing event {:?} to server", msg);
                event_writer.send(msg);
            }
            Err(e) => trace!("error reading next event to be pushed {:?}", e),
        }
    }
    trace!("done with push system");
}

fn websocket_server_system(
    task_pool: Res<IoTaskPool>,
    mut server: ResMut<WebsocketServerResource>,
) {
    match server.state {
        WsServerState::WaitingToStart => {}
        WsServerState::Starting => {
            let address = server
                .listen_address
                .take()
                .expect("Should be an address here");
            let buffer = server.new_clients_event_stream.buffer();
            let running = server.run_listen_loop.clone();
            // Task to listen to new connections
            task_pool
                .spawn(Compat::new(async move {
                    let listener = TcpListener::bind(address.as_str()).await.unwrap();
                    while *running.lock().expect("lock") {
                        trace!("main listen loop");
                        let future = listener.accept();
                        futures::pin_mut!(future);
                        match futures::poll!(future) {
                            Poll::Ready(Ok((stream, _addr))) => {
                                match stream.peer_addr() {
                                    Ok(peer) => info!("New client: {:?}", peer),
                                    Err(e) => info!("failed to obtain peer: {:?}", e),
                                }
                                match accept_async(stream).await {
                                    Ok(ws_stream) => {
                                        let client = ws_stream;
                                        buffer
                                            .lock()
                                            .expect("lock to send a new client")
                                            .push_back(client);
                                    }
                                    Err(e) => info!("Failed to upgrade websocket: {:?}", e),
                                }
                            }
                            _ => (),
                        }
                    }
                    warn!("accept new connections loop closing");
                }))
                .detach();
            server.state = WsServerState::Connected;
        }
        WsServerState::Connected => {
            trace!("1");

            if let Some(client) = server.new_clients_event_stream.next() {
                let client_id = server.generate_next_client_id();
                // let ws_to_event_channel_sender = server.ws_to_event_channel_sender.try_lock().unwrap().clone();
                let (ws_to_event_channel_sender, ws_to_event_channel_receiver) =
                    tokio::sync::mpsc::channel::<WebsocketServerEvent>(100);
                server
                    .ws_to_event_channel_receiver
                    .lock()
                    .unwrap()
                    .push(ws_to_event_channel_receiver);
                let (mut send, mut receive) = client.split();
                let running = server.run_listen_loop.clone();
                // task to listen to ws messages from client
                task_pool
                    .spawn(Compat::new(async move {
                        while *running.lock().expect("lock") {
                            match receive.next().await {
                                None => trace!("ws next is none"),
                                Some(msg) => match msg {
                                    Ok(msg) => match msg {
                                        Message::Text(msg) => {
                                            info!("server received {:?}", msg);
                                            ws_to_event_channel_sender
                                                .send(WebsocketServerEvent::OnMessage(
                                                    client_id,
                                                    msg.clone(),
                                                ))
                                                .await
                                                .unwrap();
                                            info!("done sending {:?}", msg);
                                        }

                                        o => info!("Server loop recv: {:?}", o),
                                    },
                                    Err(e) => {
                                        info!("websocket client error: {:?}", e);
                                        break;
                                    }
                                },
                            }
                        }
                        warn!("ws to event loop closing")
                    }))
                    .detach();

                let (send_ws, mut message_to_send) =
                    tokio::sync::mpsc::channel::<WebsocketClientEvent>(10);
                server
                    .message_to_be_sent_to_client_over_ws
                    .lock()
                    .unwrap()
                    .push(send_ws);
                let running = server.run_listen_loop.clone();
                // One loop per client, takes messages and sends them over websocket
                task_pool
                    .spawn(Compat::new(async move {
                        info!("Setting up send over ws loop");
                        send.send(Message::Text(format!("##CLIENT_ID## {}", client_id)))
                            .await
                            .unwrap_or_else(|e| warn!("Initial failed:{:?}", e));
                        while *running.lock().expect("lock") {
                            match message_to_send.recv().await {
                                Some(WebsocketClientEvent::OnMessage(msg)) => {
                                    info!("Sending {:?}", msg);
                                    send.send(Message::Text(msg))
                                        .await
                                        .unwrap_or_else(|e| warn!("Send failed:{:?}", e));
                                }
                                None => {
                                    warn!("channel for msg to send to ws returned none");
                                    break;
                                }
                                Some(WebsocketClientEvent::OnOpen(_client_id)) => {}
                                Some(WebsocketClientEvent::OnClose) => warn!("ws close"),
                            }
                        }
                        warn!("channel to ws loop closing");
                    }))
                    .detach();
            }
        }
    }
    trace!("End of ws system");
}

// use by the server to talk to clients
pub struct WebsocketServerResource {
    state: WsServerState,
    listen_address: Option<String>,
    pub ws_to_event_channel_receiver: Mutex<Vec<tokio::sync::mpsc::Receiver<WebsocketServerEvent>>>,
    pub new_clients_event_stream: EventStream<WebSocketStream<TcpStream>>,
    pub message_to_be_sent_to_client_over_ws:
        Mutex<Vec<tokio::sync::mpsc::Sender<WebsocketClientEvent>>>,
    run_listen_loop: Arc<Mutex<bool>>,
    next_client_id: u64,
}

impl Default for WebsocketServerResource {
    fn default() -> Self {
        Self {
            state: WsServerState::WaitingToStart,
            listen_address: None,
            ws_to_event_channel_receiver: Default::default(),
            new_clients_event_stream: Default::default(),
            message_to_be_sent_to_client_over_ws: Default::default(),
            run_listen_loop: Arc::new(Mutex::new(true)),
            next_client_id: 0,
        }
    }
}

impl WebsocketServerResource {
    pub fn generate_next_client_id(&mut self) -> u64 {
        let id = self.next_client_id;
        self.next_client_id += 1;
        id
    }
}

#[derive(Debug)]
enum WsServerState {
    WaitingToStart,
    Starting,
    Connected,
}

impl WebsocketServerResource {
    pub fn listen(&mut self, address: &str) {
        info!("Trying to listen on {}", address);
        self.listen_address = Some(address.to_string()); // todo use String
        self.state = WsServerState::Starting;
    }
    pub fn broadcast(&mut self, message: String) {
        let senders = self.message_to_be_sent_to_client_over_ws.lock().unwrap();
        for sender in &*senders {
            sender
                .blocking_send(WebsocketClientEvent::OnMessage(message.clone()))
                .unwrap_or_else(|e| warn!("Broadcast failed:{:?}", e));
        }
    }
    pub fn close(&mut self) {
        self.message_to_be_sent_to_client_over_ws
            .lock()
            .unwrap()
            .clear();
        *self.run_listen_loop.lock().unwrap() = false;
    }
}

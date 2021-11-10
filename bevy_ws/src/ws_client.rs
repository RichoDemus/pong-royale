use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use log::info;
use log::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

use crate::event_stream::EventStream;
use crate::WebsocketClientEvent;

pub struct WebsocketPlugin;

struct MessageOffset(usize); // todo remove

impl Plugin for WebsocketPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(WebsocketResource::default());
        app.add_event::<WebsocketClientEvent>();
        app.add_system(setup_websocket_system.system());
        app.insert_resource(MessageOffset(0));
        app.add_system(write_websocket_event_to_client.system());
    }
}

fn write_websocket_event_to_client(
    mut event_writer: EventWriter<WebsocketClientEvent>,
    mut resource: ResMut<WebsocketResource>,
) {
    while let Some(msg) = resource.ws_to_event_channel_receiver.next() {
        info!("system pusing event {:?} to client", msg);
        event_writer.send(msg);
    }
}

fn setup_websocket_system(mut resource: ResMut<WebsocketResource>, task_pool: Res<IoTaskPool>) {
    if let Some(address) = resource.address_to_connect_to.take() {
        let ws: WebSocket = WebSocket::new(address.as_str()).unwrap();
        {
            // On Error
            let _buffer = resource.ws_to_event_channel_receiver.buffer().clone();
            let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
                warn!("WS error: {:?}", e);
            }) as Box<dyn FnMut(ErrorEvent)>);
            ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();
        }

        {
            // On Message
            let buffer = resource.ws_to_event_channel_receiver.buffer().clone();
            let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let str: String = txt.into();
                    info!("Client received: {:?}", str);
                    if str.contains("##CLIENT_ID##") {
                        let mut split = str.split(" ");
                        split.next();
                        let id = split.next();
                        let id = id.unwrap();
                        buffer
                            .lock()
                            .expect("aquire lock")
                            .push_back(WebsocketClientEvent::OnOpen(id.parse().unwrap()));
                    } else {
                        buffer
                            .lock()
                            .expect("aquire lock")
                            .push_back(WebsocketClientEvent::OnMessage(str));
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

        {
            // on close
            let buffer = resource.ws_to_event_channel_receiver.buffer().clone();
            let onclose_callback = Closure::wrap(Box::new(move |_| {
                buffer
                    .lock()
                    .expect("aquire lock")
                    .push_back(WebsocketClientEvent::OnClose);
            }) as Box<dyn FnMut(JsValue)>);
            ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();
        }

        {
            // on open
            let _buffer = resource.ws_to_event_channel_receiver.buffer().clone();
            let onopen_callback = Closure::wrap(Box::new(move |_| {
                info!("Ws opened");
            }) as Box<dyn FnMut(JsValue)>);
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        }

        // todo have event stream impl clone and use that here instead
        let message_to_be_sent_over_ws = resource.message_to_be_sent_over_ws.buffer();
        let running = resource.run_listen_loop.clone();
        task_pool
            .spawn(async move {
                pub fn next_event(
                    buffer: Arc<Mutex<VecDeque<String>>>,
                ) -> impl Future<Output = Option<String>> {
                    use futures_util::future::poll_fn;
                    poll_fn(move |_cx| {
                        let mut buffer = buffer
                            .lock()
                            .expect("EventStream.next_event failed to obtain lock");
                        match buffer.pop_front() {
                            Some(event) => Poll::Ready(Some(event)),
                            None => Poll::Ready(None),
                        }
                    })
                }

                while *running.lock().expect("lock") {
                    if let Some(msg) = next_event(message_to_be_sent_over_ws.clone()).await {
                        ws.send_with_str(msg.as_str())
                            .expect("wsclient.send failed");
                    }
                    let _ = wasm_bindgen_futures::JsFuture::from(sleep(10)).await;
                }
            })
            .detach();
    }
}

#[wasm_bindgen]
pub fn sleep(ms: i32) -> js_sys::Promise {
    js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    })
}

// use by the client to talk to server
pub struct WebsocketResource {
    pub ws_to_event_channel_receiver: EventStream<WebsocketClientEvent>,
    address_to_connect_to: Option<String>,
    pub message_to_be_sent_over_ws: EventStream<String>,
    run_listen_loop: Arc<Mutex<bool>>,
}

impl Default for WebsocketResource {
    fn default() -> Self {
        Self {
            address_to_connect_to: None,
            ws_to_event_channel_receiver: Default::default(),
            message_to_be_sent_over_ws: Default::default(),
            run_listen_loop: Arc::new(Mutex::new(true)),
        }
    }
}

impl WebsocketResource {
    pub fn open(&mut self, address: &str) {
        self.address_to_connect_to.replace(address.to_string());
    }
    pub fn broadcast(&self, message: String) {
        self.message_to_be_sent_over_ws
            .buffer()
            .lock()
            .expect("lock")
            .push_back(message);
    }
    pub fn close(&self) {
        *self.run_listen_loop.lock().unwrap() = false;
    }
}

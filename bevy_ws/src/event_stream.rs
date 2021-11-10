use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures_util::future::poll_fn;

pub struct EventStream<T> {
    buffer: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Default for EventStream<T> {
    fn default() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl<T> EventStream<T> {
    pub fn buffer(&self) -> Arc<Mutex<VecDeque<T>>> {
        self.buffer.clone()
    }

    pub fn next_event(&mut self) -> impl Future<Output = Option<T>> {
        let buffer = self.buffer.clone();
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

    pub fn next(&mut self) -> Option<T> {
        self.buffer
            .lock()
            .expect("EventStream.next failed to obtain lock")
            .pop_front()
    }
}

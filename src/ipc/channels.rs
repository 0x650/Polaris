use crate::locks::mutex::Mutex;
use crate::sched::dispatch::{Dispatcher, Event, wait_on_single_object, DispatcherObject};
use crate::status_codes::{PxResult, PxStatus};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct Message {
    data: Box<[u8]>,
}

impl Message {
    fn new(data: &[u8]) -> Self {
        Message { data: Box::from(data) }
    }
}

struct Inner {
    messages: VecDeque<Message>,
    peer: Weak<Channel>,
}

pub struct Channel {
    inner: Mutex<Inner>,
    event: Arc<Event>,
}

impl Channel {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                messages: VecDeque::new(),
                peer: Weak::new(),
            }),
            event: Arc::new(Event::new()),
        }
    }

    pub fn new_pair() -> (Arc<Channel>, Arc<Channel>) {
        let (a, b) = (Arc::new(Channel::new()), Arc::new(Channel::new()));
        a.inner.lock().peer = Arc::downgrade(&b);
        b.inner.lock().peer = Arc::downgrade(&a);
        (a, b)
    }

    pub fn send(&self, data: &[u8]) -> PxResult<()> {
        let msg = Message::new(data);

        let peer = self.inner.lock().peer.upgrade();
        let Some(peer) = peer else {
            return Err(PxStatus::PeerClosed);
        };

        let mut g = peer.inner.lock();
        g.messages.push_back(msg);
        peer.event.trigger(true);
        Ok(())
    }

    pub fn try_receive(&self, buffer: &mut [u8]) -> PxResult<usize> {
        let msg = {
            let mut g = self.inner.lock();

            let Some(front) = g.messages.front() else {
                return Err(if g.peer.strong_count() == 0 {
                    PxStatus::PeerClosed
                } else {
                    PxStatus::ShouldWait
                });
            };

            if buffer.len() < front.data.len() {
                return Err(PxStatus::BufferTooSmall);
            }

            let m = g.messages.pop_front().unwrap();

            if g.messages.is_empty() && g.peer.strong_count() != 0 {
                self.event.trigger(false);
            }
            m
        };

        let len = msg.data.len();
        buffer[..len].copy_from_slice(&msg.data);
        Ok(len)
    }

    pub fn receive(self: &Arc<Self>, buffer: &mut [u8]) -> PxResult<usize> {
        loop {
            match self.try_receive(buffer) {
                Err(PxStatus::ShouldWait) => {
                    let r = wait_on_single_object(Arc::new(self.event.clone().into()), usize::MAX);
                    if r != PxStatus::Success {
                        return Err(r);
                    }
                }
                other => return other,
            }
        }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        let (peer, queued) = {
            let mut g = self.inner.lock();
            (g.peer.upgrade(), core::mem::take(&mut g.messages))
        };
        if let Some(peer) = peer {
            let _g = peer.inner.lock();
            peer.event.trigger(true);
        }
        drop(queued);
    }
}

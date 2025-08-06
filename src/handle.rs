use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub mod blocking;

pub struct JoinHandle<T>(Arc<Mutex<Option<T>>>);

impl<T: 'static + Send> JoinHandle<T> {
    pub fn new(inner: Arc<Mutex<Option<T>>>) -> Self {
        Self(inner)
    }

    pub fn nonblocking_pool(&self) -> Option<T> {
        if let Ok(mut lock) = self.0.try_lock() {
            return lock.take();
        }

        None
    }
}

impl<T: 'static + Send> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.nonblocking_pool() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

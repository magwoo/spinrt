use std::sync::{Arc, Condvar, Mutex};

pub struct BlockingJoinHandle<T>(Arc<(Mutex<Option<T>>, Condvar)>);

impl<T: 'static + Send> BlockingJoinHandle<T> {
    pub(crate) fn new(lock: Arc<(Mutex<Option<T>>, Condvar)>) -> Self {
        Self(lock)
    }

    pub fn poll_nonblocking(&self) -> Option<T> {
        if let Ok(mut lock) = self.0.0.try_lock() {
            return lock.take();
        }

        None
    }

    pub fn join(self) -> T {
        loop {
            let guard = self.0.0.lock().unwrap();
            let mut lock = self.0.1.wait(guard).unwrap();

            if let Some(result) = lock.take() {
                return result;
            }
        }
    }
}

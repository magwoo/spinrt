use core::future::Future;
use crossbeam_deque::Injector as SharedQueue;
use crossbeam_deque::Steal;
use crossbeam_deque::Stealer;
use crossbeam_deque::Worker as WorkerQueue;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;
use std::time::Instant;

pub use join::JoinHandle;

mod join;

pub mod time;

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
}

struct Worker {
    local_queue: WorkerQueue<Task>,
    shared_queue: Arc<SharedQueue<Task>>,
    stealers: Arc<[Stealer<Task>]>,
}

#[derive(Default)]
pub struct Runtime {
    shared_queue: Arc<SharedQueue<Task>>,
}

impl Runtime {
    pub fn new(workers_num: usize) -> Self {
        let shared_queue = Arc::new(SharedQueue::default());
        let workers_queues = (0..workers_num)
            .map(|_| WorkerQueue::new_lifo())
            .collect::<Vec<_>>();

        let stealers = workers_queues
            .iter()
            .map(|q| q.stealer())
            .collect::<Arc<[_]>>();

        workers_queues
            .into_iter()
            .map(|q| Worker {
                local_queue: q,
                shared_queue: shared_queue.clone(),
                stealers: stealers.clone(),
            })
            .for_each(|w| {
                std::thread::spawn(|| w.event_loop());
            });

        Self { shared_queue }
    }

    pub fn spawn<T: 'static + Send>(
        &self,
        future: impl Future<Output = T> + 'static + Send,
    ) -> JoinHandle<T> {
        let result = Arc::new(Mutex::new(None));

        let result_cloned = Arc::clone(&result);
        let wrapped_future = async move {
            let result = future.await;
            *result_cloned.lock().unwrap() = Some(result);
        };

        self.shared_queue.push(Task::new(wrapped_future));

        JoinHandle::new(result)
    }

    pub fn block_on(&self, future: impl Future<Output = ()> + 'static + Send) {
        let handle = self.spawn(future);

        while handle.is_ready().is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Worker {
    const MIN_PASS_TIMEOUT: Duration = Duration::from_micros(5);
    const MAX_PASS_TIMEOUT: Duration = Duration::from_millis(1);

    fn event_loop(self) {
        let mut pass_timeout = Self::MIN_PASS_TIMEOUT;
        let mut last_pass = Instant::now();

        loop {
            let mut solved = 0;
            while let Some(task) = self.search_task() {
                task.run(&self.local_queue);

                solved += 1;
            }

            pass_timeout = match solved {
                1.. => Self::MIN_PASS_TIMEOUT,
                _ => (pass_timeout * 2).min(Self::MAX_PASS_TIMEOUT),
            };

            let pass_duration = last_pass.elapsed();
            if pass_duration < pass_timeout {
                std::thread::sleep(pass_timeout - pass_duration);
            }

            last_pass = Instant::now();
            std::thread::yield_now();
        }
    }

    fn search_task(&self) -> Option<Task> {
        self.local_queue.pop().or_else(|| {
            if let Steal::Success(task) = self.shared_queue.steal() {
                Some(task)
            } else {
                self.stealers.iter().find_map(|s| match s.steal() {
                    Steal::Success(task) => Some(task),
                    _ => None,
                })
            }
        })
    }
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static + Send) -> Self {
        Self {
            future: Box::pin(future),
        }
    }

    fn run(mut self, local_queue: &WorkerQueue<Self>) {
        let waker = Waker::noop();

        // FIXME: move result return here
        match self.future.as_mut().poll(&mut Context::from_waker(waker)) {
            Poll::Ready(_) => println!("some task is done"),
            Poll::Pending => local_queue.push(self),
        }
    }
}

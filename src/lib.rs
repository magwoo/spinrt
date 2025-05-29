use core::future::Future;
use crossbeam_deque::Injector as SharedQueue;
use crossbeam_deque::Worker as WorkerQueue;
use crossbeam_deque::{Steal, Stealer};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

pub use handle::JoinHandle;

mod handle;

pub mod macros;
pub mod net;
pub mod time;

static SHARED_QUEUE: OnceLock<SharedQueue<Task>> = OnceLock::new();

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
}

struct Worker {
    local_queue: WorkerQueue<Task>,
    stealers: Arc<[Stealer<Task>]>,
}

pub fn create(workers_num: usize) {
    SHARED_QUEUE.get_or_init(SharedQueue::default);

    let workers_queues = (0..workers_num)
        .map(|_| WorkerQueue::new_fifo())
        .collect::<Vec<_>>();

    let stealers = workers_queues
        .iter()
        .map(|q| q.stealer())
        .collect::<Arc<[_]>>();

    workers_queues
        .into_iter()
        .map(|q| Worker {
            local_queue: q,
            stealers: stealers.clone(),
        })
        .for_each(|w| {
            std::thread::spawn(|| w.event_loop());
        });
}

pub fn spawn<T: 'static + Send>(future: impl Future<Output = T> + 'static + Send) -> JoinHandle<T> {
    let shared_queue = SHARED_QUEUE.get().expect("Runtime is not created");

    let result = Arc::new(Mutex::new(None));

    let result_cloned = Arc::clone(&result);
    let wrapped_future = async move {
        let result = future.await;
        *result_cloned.lock().unwrap() = Some(result);
    };

    shared_queue.push(Task::new(wrapped_future));

    JoinHandle::new(result)
}

pub fn block_on(future: impl Future<Output = ()> + 'static + Send) {
    let handle = spawn(future);

    while handle.is_ready().is_none() {
        std::thread::sleep(Duration::from_millis(1));
    }
}

impl Worker {
    const MIN_PASS_TIMEOUT: Duration = Duration::from_micros(50);
    const MAX_PASS_TIMEOUT: Duration = Duration::from_millis(1);

    fn event_loop(self) -> ! {
        let mut pass_timeout = Self::MIN_PASS_TIMEOUT;
        let mut last_pass = Instant::now();

        loop {
            let mut solved = 0;
            let mut pending_tasks = Vec::new();

            while let Some(task) = self.search_tasks() {
                let is_solved = task.run(&mut pending_tasks);

                if is_solved {
                    solved += 1;
                }
            }

            pending_tasks.into_iter().for_each(|task| {
                self.local_queue.push(task);
            });

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

    fn search_tasks(&self) -> Option<Task> {
        let _ = SHARED_QUEUE
            .get()
            .expect("Runtime is not created")
            .steal_batch(&self.local_queue);

        self.local_queue.pop().or_else(|| {
            self.stealers.iter().find_map(|s| match s.steal() {
                Steal::Success(task) => Some(task),
                _ => None,
            })
        })
    }
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static + Send) -> Self {
        Self {
            future: Box::pin(future),
        }
    }

    fn run(mut self, pendings: &mut Vec<Self>) -> bool {
        let waker = Waker::noop();

        // FIXME: move result return here
        match self.future.as_mut().poll(&mut Context::from_waker(waker)) {
            Poll::Ready(_) => return true,
            Poll::Pending => pendings.push(self),
        }

        false
    }
}

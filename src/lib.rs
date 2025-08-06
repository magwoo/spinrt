use core::future::Future;
use crossbeam_deque::Injector as SharedQueue;
use crossbeam_deque::Worker as WorkerQueue;
use crossbeam_deque::{Steal, Stealer};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Condvar;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

pub use handle::JoinHandle;

use crate::handle::blocking::BlockingJoinHandle;

mod handle;

pub mod macros;
pub mod net;
pub mod time;

type BlockingTask = dyn FnOnce() + 'static + Send;

static SHARED_QUEUE: OnceLock<SharedQueue<Task>> = OnceLock::new();
static BLOCKING_QUEUE: OnceLock<Arc<Mutex<VecDeque<Box<BlockingTask>>>>> = OnceLock::new();
static BLOCKUNG_QUEUE_CONDVAR: Condvar = Condvar::new();

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
}

struct Worker {
    local_queue: WorkerQueue<Task>,
    stealers: Arc<[Stealer<Task>]>,
}

pub fn create(workers_num: usize) {
    SHARED_QUEUE.get_or_init(SharedQueue::default);
    BLOCKING_QUEUE.get_or_init(Arc::default);

    (0..workers_num).for_each(|_| {
        std::thread::spawn(|| {
            loop {
                let guard = BLOCKING_QUEUE
                    .get()
                    .expect("must be inited")
                    .lock()
                    .unwrap();

                let (mut queue, _) = BLOCKUNG_QUEUE_CONDVAR
                    .wait_timeout(guard, Duration::from_millis(1))
                    .unwrap();

                let task = match queue.pop_front() {
                    Some(task) => task,
                    None => continue,
                };

                drop(queue);

                task();
            }
        });
    });

    let worker_queues = (0..workers_num)
        .map(|_| WorkerQueue::new_fifo())
        .collect::<Vec<_>>();

    let stealers = worker_queues
        .iter()
        .map(|q| q.stealer())
        .collect::<Arc<[_]>>();

    worker_queues
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

pub fn spawn_blocking<F: FnOnce() -> T + 'static + Send, T: 'static + Send + Sync>(
    f: F,
) -> BlockingJoinHandle<T> {
    let queue = BLOCKING_QUEUE.get().expect("must be initialized");

    let mut queue = queue.lock().unwrap();
    let lock = Arc::new((Mutex::new(None), Condvar::new()));

    let lock_clone = Arc::clone(&lock);
    queue.push_back(Box::new(move || {
        let exec_result = f();
        lock_clone.0.lock().unwrap().replace(exec_result);
        lock_clone.1.notify_one();
    }));

    drop(queue);
    BLOCKUNG_QUEUE_CONDVAR.notify_one();

    BlockingJoinHandle::new(lock)
}

pub fn block_on(future: impl Future<Output = ()> + 'static + Send) {
    let handle = spawn(future);

    while handle.nonblocking_pool().is_none() {
        std::thread::sleep(Duration::from_millis(1));
    }
}

impl Worker {
    const MIN_PASS_TIMEOUT: Duration = Duration::from_micros(500);
    const MAX_PASS_TIMEOUT: Duration = Duration::from_millis(25);

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

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use super::waker::{ReadyQueue, TaskWaker};

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone)]
pub struct SpawnHandle {
    queue: Arc<Mutex<VecDeque<Task>>>,
}

impl SpawnHandle {
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        self.queue.lock().unwrap().push_back(Box::pin(future));
    }
}

pub struct Executor {
    tasks: HashMap<u64, Task>,
    ready_queue: ReadyQueue,
    spawn_queue: Arc<Mutex<VecDeque<Task>>>,
    next_id: u64,
}

impl Executor {
    pub fn new() -> (Self, SpawnHandle) {
        let spawn_queue = Arc::new(Mutex::new(VecDeque::new()));

        let executor = Executor {
            tasks: HashMap::new(),
            ready_queue: Arc::new(Mutex::new(VecDeque::new())),
            spawn_queue: spawn_queue.clone(),
            next_id: 0,
        };

        let handle = SpawnHandle { queue: spawn_queue };

        (executor, handle)
    }

    fn register_task(&mut self, task: Task) {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(id, task);
        self.ready_queue.lock().unwrap().push_back(id);
    }

    pub fn spawn(&mut self, future: impl Future<Output = ()> + Send + 'static) {
        self.register_task(Box::pin(future));
    }

    pub fn run(&mut self) {
        loop {
            let new_tasks: Vec<Task> = {
                let mut queue = self.spawn_queue.lock().unwrap();
                queue.drain(..).collect()
            };
            for task in new_tasks {
                self.register_task(task);
            }

            let ready: Vec<u64> = {
                let mut queue = self.ready_queue.lock().unwrap();
                queue.drain(..).collect()
            };

            if ready.is_empty() && self.tasks.is_empty() {
                break;
            }

            // if nothing is ready yet self.tasks ain't empty, sleep until called by reactor
            if ready.is_empty() {
                std::thread::park();
                continue;
            }

            for task_id in ready {
                let Some(task) = self.tasks.get_mut(&task_id) else {
                    continue; // task already completed
                };

                // waker will re-queue this task id when called
                let waker = TaskWaker {
                    task_id,
                    ready_queue: self.ready_queue.clone(),
                    thread: std::thread::current(),
                }
                .into_waker();

                let mut cx = Context::from_waker(&waker);

                match task.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => {
                        // task is done so we remove it
                        self.tasks.remove(&task_id);
                    }
                    Poll::Pending => {
                        // task is waiting on IO, waker will re-queue it
                    }
                }
            }
        }
    }
}

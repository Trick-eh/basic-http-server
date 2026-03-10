use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{RawWaker, RawWakerVTable, Waker},
    thread::Thread,
};

pub type ReadyQueue = Arc<Mutex<VecDeque<u64>>>;

pub struct TaskWaker {
    pub task_id: u64,
    pub ready_queue: ReadyQueue,
    pub thread: Thread,
}

impl TaskWaker {
    pub fn into_waker(self) -> Waker {
        let arc = Arc::new(self);
        let raw = raw_waker(arc);
        unsafe { Waker::from_raw(raw) }
    }
}

fn raw_waker(arc: Arc<TaskWaker>) -> RawWaker {
    let ptr = Arc::into_raw(arc) as *const ();
    RawWaker::new(ptr, &VTABLE)
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    // clone
    |ptr| unsafe {
        let arc = Arc::from_raw(ptr as *const TaskWaker);
        let cloned = arc.clone();
        std::mem::forget(arc); // don't drop the original arc
        raw_waker(cloned)
    },
    // wake (consumes the waker)
    |ptr| unsafe {
        let arc = Arc::from_raw(ptr as *const TaskWaker);
        let task_id = arc.task_id;
        let queue = arc.ready_queue.clone();
        let thread = arc.thread.clone();
        drop(arc);
        queue.lock().unwrap().push_back(task_id);
        thread.unpark();
    },
    // wake_by_ref (won't consume)
    |ptr| unsafe {
        let arc = Arc::from_raw(ptr as *const TaskWaker);
        let task_id = arc.task_id;
        let queue = arc.ready_queue.clone();
        let thread = arc.thread.clone();
        std::mem::forget(arc);
        queue.lock().unwrap().push_back(task_id);
        thread.unpark();
    },
    // drop
    |ptr| unsafe {
        drop(Arc::from_raw(ptr as *const TaskWaker));
    },
);

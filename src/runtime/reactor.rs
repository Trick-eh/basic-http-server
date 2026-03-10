use libc::{
    EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, EPOLLIN, EPOLLOUT, epoll_create1, epoll_ctl,
    epoll_event, epoll_wait,
};
use std::{
    collections::HashMap,
    os::unix::io::RawFd,
    sync::{Arc, Mutex},
    task::Waker,
    thread,
};

// Singleton pattern, there shouldn't be more than one reactor
static REACTOR: std::sync::OnceLock<Reactor> = std::sync::OnceLock::new();

pub fn reactor() -> &'static Reactor {
    REACTOR.get_or_init(Reactor::new)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interest {
    Readable,
    Writable,
}

impl Interest {
    fn to_epoll_events(self) -> u32 {
        match self {
            Interest::Readable => EPOLLIN as u32,
            Interest::Writable => EPOLLOUT as u32,
        }
    }
}

type Registry = Arc<Mutex<HashMap<(RawFd, Interest), Waker>>>;

pub struct Reactor {
    epoll_fd: RawFd,
    registry: Registry,
}

impl Reactor {
    fn new() -> Self {
        let epoll_fd = unsafe { epoll_create1(0) };
        assert!(epoll_fd >= 0, "failed to create epoll instance");

        let reactor = Reactor {
            epoll_fd,
            registry: Arc::new(Mutex::new(HashMap::new())),
        };

        // spawn background thread that blocks on epoll_wait and wakes futures when IO is ready
        reactor.spawn_event_loop();

        reactor
    }

    pub fn register(&self, fd: RawFd, interest: Interest, waker: Waker) {
        let mut registry = self.registry.lock().unwrap();
        let already_watching = registry.keys().any(|(f, _)| *f == fd);

        registry.insert((fd, interest), waker);

        let mut event = epoll_event {
            events: interest.to_epoll_events(),
            u64: fd as u64,
        };

        let op = if already_watching {
            EPOLL_CTL_MOD
        } else {
            EPOLL_CTL_ADD
        };

        unsafe {
            epoll_ctl(self.epoll_fd, op, fd, &mut event);
        }
    }

    pub fn deregister(&self, fd: RawFd) {
        let mut registry = self.registry.lock().unwrap();
        registry.retain(|(f, _), _| *f != fd);

        unsafe {
            epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        }
    }

    fn spawn_event_loop(&self) {
        let epoll_fd = self.epoll_fd;
        let registry = self.registry.clone();

        thread::spawn(move || {
            let mut events = vec![epoll_event { events: 0, u64: 0 }; 1024];

            loop {
                // blocked until at least one fd is read (timeout = -1 is block forever)
                let n =
                    unsafe { epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, -1) };

                if n < 0 {
                    eprintln!("epoll_wait error");
                    continue;
                }

                let registry: std::sync::MutexGuard<HashMap<(RawFd, Interest), Waker>> =
                    registry.lock().unwrap();

                for event in &events[..n as usize] {
                    let fd = event.u64 as RawFd;
                    let events = event.events;

                    if events & EPOLLIN as u32 != 0 {
                        if let Some(waker) = registry.get(&(fd, Interest::Readable)) {
                            waker.wake_by_ref();
                        }
                    }

                    if events & EPOLLOUT as u32 != 0 {
                        if let Some(waker) = registry.get(&(fd, Interest::Writable)) {
                            waker.wake_by_ref();
                        }
                    }
                }
            }
        });
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.epoll_fd);
        }
    }
}

use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct LockWaiter {
    lock: Arc<Mutex<()>>,
    waiter: Arc<Condvar>
}

impl LockWaiter {
    pub fn new() -> LockWaiter{
        LockWaiter {
            lock: Arc::new(Mutex::new(())),
            waiter: Arc::new(Condvar::new()),
        }
    }

    pub fn wait(self: &Self) {
        self.waiter.wait(self.lock.lock().unwrap()).unwrap();
    }

    pub fn notify(self: &Self) {
        self.waiter.notify_all();
    }
}
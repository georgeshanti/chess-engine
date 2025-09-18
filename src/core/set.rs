use std::{collections::BTreeSet, hash::Hash, sync::{Arc, Condvar, Mutex}};

#[derive(Clone)]
pub struct Set<T: Ord> {
    set: Arc<Mutex<BTreeSet<T>>>,
    waiter: Arc<Condvar>,
}

impl<T: Ord> Set<T> {
    pub fn new() -> Self {
        Set {
            set: Arc::new(Mutex::new(BTreeSet::new())),
            waiter: Arc::new(Condvar::new()),
        }
    }

    pub fn add(&self, values: Vec<T>) {
        let mut set = self.set.lock().unwrap();
        if set.is_empty() {
            for value in values {
                set.insert(value);
            }
            self.waiter.notify_all();
        } else {
            for value in values {
                set.insert(value);
            }
        }
    }

    pub fn pop(&self) -> T {
        let mut set = self.set.lock().unwrap();
        let mut value = set.pop_last();
        loop {
            match value {
                None => {
                    drop(value);
                    // println!("Waiting for head");
                    set = self.waiter.wait(set).unwrap();
                    value = set.pop_last();
                },
                Some(val) => {
                    return val;
                }
            }
        }
    }
}
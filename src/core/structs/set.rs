use std::{collections::BTreeSet, sync::{Arc, Condvar, Mutex, RwLock}};

#[derive(Clone)]
pub struct Set<T: Ord> {
    set: Arc<Mutex<BTreeSet<T>>>,
    waiter: Arc<Condvar>,
    pub length: Arc<RwLock<usize>>,
}

impl<T: Ord> Set<T> {
    pub fn new() -> Self {
        Set {
            set: Arc::new(Mutex::new(BTreeSet::new())),
            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        }
    }

    pub fn add(&self, values: Vec<T>) {
        let mut set = self.set.lock().unwrap();
        let values_len = values.len();
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
        let mut len = self.length.write().unwrap();
        *len = *len + values_len;
    }

    pub fn pop_first(&self) -> T {
        let mut set = self.set.lock().unwrap();
        let mut value = set.pop_first();
        loop {
            match value {
                None => {
                    drop(value);
                    // println!("Waiting for head");
                    set = self.waiter.wait(set).unwrap();
                    value = set.pop_first();
                },
                Some(val) => {
                    let mut len = self.length.write().unwrap();
                    *len = *len - 1;
                    return val;
                }
            }
        }
    }

    pub fn pop_last(&self) -> T {
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
                    let mut len = self.length.write().unwrap();
                    *len = *len - 1;
                    return val;
                }
            }
        }
    }
}
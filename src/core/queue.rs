use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct QueueNode<T> {
    pub value: Vec<T>,
    pub next: Arc<Mutex<Option<QueueNode<T>>>>,
}

#[derive(Clone)]
pub struct Queue<T> {
    pub head: Arc<Mutex<Arc<Mutex<Option<QueueNode<T>>>>>>,
    pub tail: Arc<Mutex<Arc<Mutex<Option<QueueNode<T>>>>>>,

    waiter: Arc<Condvar>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            head: Arc::new(Mutex::new(Arc::new(Mutex::new(None)))),
            tail: Arc::new(Mutex::new(Arc::new(Mutex::new(None)))),

            waiter: Arc::new(Condvar::new()),
        }
    }
}

pub trait QueueTrait<T> {
    fn queue(&self, value: Vec<T>);
    fn dequeue(&self) -> T;
}

impl<T> QueueTrait<T> for Queue<T> {
    fn queue(&self, value: Vec<T>) {
        if value.is_empty() {
            return;
        }
        let new_node = Arc::new(Mutex::new(Some(QueueNode { value, next: Arc::new(Mutex::new(None)) })));

        let mut should_update_head = false;
        let mut tail_pointer = self.tail.lock().unwrap();
        {
            let mut tail = tail_pointer.lock().unwrap();

            match *tail {
                Some(ref mut tail) => {
                    // println!("Queueing: Some");
                    tail.next = new_node.clone();
                }
                None => {
                    // println!("Queueing: None");
                    should_update_head = true;
                }
            }
        }

        *tail_pointer = new_node.clone();
        drop(tail_pointer);
        if should_update_head {
            println!("Updating head");
            let mut head_pointer = self.head.lock().unwrap();
            *head_pointer = new_node.clone();
            self.waiter.notify_all();
        }

    }

    fn dequeue(&self) -> T {
        let mut head_pointer = self.head.lock().unwrap();
        // let start = SystemTime::now();
        let mut head = {
            let mut scoped_head = head_pointer.lock().unwrap();
            loop {
                match *scoped_head {
                    None => {
                        drop(scoped_head);
                        println!("Waiting for head");
                        head_pointer = self.waiter.wait(head_pointer).unwrap();
                        scoped_head = head_pointer.lock().unwrap();
                    },
                    Some(_) => {
                        break scoped_head;
                    }
                }
            }
        };
        let mut new_head: Option<Arc<Mutex<Option<QueueNode<T>>>>> = None;
        let mut should_update_tail = false;
        let return_value: T;
        {
            if let Some(ref mut head_node) = *head {
                let value = head_node.value.pop().unwrap();
                if head_node.value.is_empty() {
                    let next = head_node.next.clone();
                    let next_guard = next.lock().unwrap();
                    if next_guard.is_none() {
                        should_update_tail = true;
                    }
                    drop(next_guard);
                    new_head = Some(next);
                }
                return_value = value;
            } else {
                panic!("Head unexpectedly empty");
            };
        }
        drop(head);
        if let Some(new_head) = new_head {
            *head_pointer = new_head.clone();
            if should_update_tail {
                let mut tail_pointer = self.tail.lock().unwrap();
                *tail_pointer = new_head.clone();
            }
        }

        // let end = SystemTime::now();
        // let elapsed = end.duration_since(start).unwrap().as_nanos();
        // if elapsed > 0 {
        //     println!("it took {}ns", elapsed);
        // }
        return_value
    }
}
use std::{sync::{Arc, Condvar, Mutex}};

#[derive(Clone)]
struct QueueNode<T> {
    value: Vec<T>,
    next: Arc<Mutex<Option<QueueNode<T>>>>,
}

pub struct Queue<T> {
    head: Arc<Mutex<Option<QueueNode<T>>>>,
    tail: Arc<Mutex<Option<QueueNode<T>>>>,

    waiter: Arc<Condvar>,
    mutex: Arc<Mutex<()>>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            head: Arc::new(Mutex::new(None)),
            tail: Arc::new(Mutex::new(None)),

            waiter: Arc::new(Condvar::new()),
            mutex: Arc::new(Mutex::new(())),
        }
    }
}

pub trait QueueTrait<T> {
    fn queue(&self, value: Vec<T>);
    fn dequeue(&self) -> T;
}

impl<T> QueueTrait<T> for Arc<Mutex<Queue<T>>> {
    fn queue(&self, value: Vec<T>) {
        // println!("Queueing");
        let cloned_queue = self.clone();
        // println!("Cloned queue");
        let mut locked_queue = cloned_queue.lock().unwrap();
        // println!("Read queue");
        let new_node = Arc::new(Mutex::new(Some(QueueNode { value, next: Arc::new(Mutex::new(None)) })));

        let mut update_head = false;
        {
            let mut tail = locked_queue.tail.lock().unwrap();
            // println!("Got tail");

            match *tail {
                Some(ref mut tail) => {
                    // println!("Queueing: Some");
                    tail.next = new_node.clone();
                }
                None => {
                    // println!("Queueing: None");
                    update_head = true;
                }
            }
        }

        locked_queue.tail = new_node.clone();
        if update_head {
            locked_queue.head = new_node.clone();
            locked_queue.waiter.notify_all();
        }

        // println!("Queued");
    }

    fn dequeue(&self) -> T {
        let cloned_queue = self.clone();
        let mut locked_queue = cloned_queue.lock().unwrap();
        {
            let mut head = locked_queue.head.lock().unwrap();
            loop {
                match *head {
                    None => {
                        // println!("Dequeueing: None");
                        let waiter = locked_queue.waiter.clone();
                        let mutex = locked_queue.mutex.clone();

                        drop(head);
                        drop(locked_queue);

                        let _unused = waiter.wait(mutex.lock().unwrap()).unwrap();
                        // println!("Dequeueing: None: Waiting");

                        locked_queue = cloned_queue.lock().unwrap();
                        head = locked_queue.head.lock().unwrap();
                    },
                    Some(_) => {
                        break;
                    }
                }
            };
        }
        let mut new_head: Option<Arc<Mutex<Option<QueueNode<T>>>>> = None;
        let mut should_update_tail = false;
        let return_value: T;
        {
            let mut head = locked_queue.head.lock().unwrap();
            if let Some(ref mut head_node) = *head {
                if head_node.value.is_empty() {
                    panic!("Head node is empty");
                }
                let value = head_node.value.pop().unwrap();
                if head_node.value.is_empty() {
                    let next = head_node.next.clone();
                    if next.lock().unwrap().is_none() {
                        should_update_tail = true;
                    }
                    new_head = Some(next);
                }
                // let mut writable_head = readable_queue.head.write().unwrap();
                // let value = writable_head.as_mut().unwrap().value.pop();
                return_value = value;
            } else {
                panic!("Head unexpectedly empty");
            };
        }

        if let Some(new_head) = new_head {
            locked_queue.head = new_head.clone();
            if should_update_tail {
                locked_queue.tail = new_head.clone();
            }
        }

        return_value
    }
}
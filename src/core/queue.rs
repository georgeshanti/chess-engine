use std::{collections::HashSet, sync::{Arc, Condvar, Mutex, RwLock}};

use crate::core::board::Board;

#[derive(Clone)]
pub struct DistributedQueue<T> {
    pub size: usize,
    pub current_node: Arc<Mutex<usize>>,
    pub queues: Vec<Queue<T>>,
}

impl<T> DistributedQueue<T> {

    pub fn new(size: usize) -> Self {
        let mut queue = DistributedQueue {
            size,
            current_node: Arc::new(Mutex::new(0)),
            queues: Vec::with_capacity(size),
        };
        for _ in 0..size {
            queue.queues.push(Queue::new());
        }
        return queue;
    }

    pub fn queue(&self, value: Vec<T>) {
        // let current_node = {
        //     let mut current_node = self.current_node.lock().unwrap();
        //     let index_to_queue_to = *current_node;
        //     *current_node = (*current_node + 1) % self.size;
        //     index_to_queue_to
        // };
        let current_node = {
            let mut shortest_queue_length: Option<usize> = None;
            let mut shortest_queue_length_index = 0;
            for i in 0..self.queues.len() {
                let queue_length = {
                    *(self.queues[i].length.read().unwrap())
                };
                match shortest_queue_length {
                    None => {
                        shortest_queue_length = Some(queue_length);
                        shortest_queue_length_index = i;
                    }
                    Some(value) => {
                        if queue_length < value {
                            shortest_queue_length = Some(queue_length);
                            shortest_queue_length_index = i;
                        }
                    }
                }
            }
            shortest_queue_length_index
        };
        self.queues[current_node].queue(value);
    }

    pub fn dequeue(&self, i: usize) -> T {
        self.queues[i].dequeue()
    }
}

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
    pub length: Arc<RwLock<usize>>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            head: Arc::new(Mutex::new(Arc::new(Mutex::new(None)))),
            tail: Arc::new(Mutex::new(Arc::new(Mutex::new(None)))),

            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        }
    }

    pub fn queue(&self, value: Vec<T>) {
        if value.is_empty() {
            return;
        }
        let len = value.len();
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
        {
            let mut length = self.length.write().unwrap();
            *length = *length + len;
        }
        if should_update_head {
            // println!("Updating head");
            let mut head_pointer = self.head.lock().unwrap();
            *head_pointer = new_node.clone();
            self.waiter.notify_all();
        }
    }

    pub fn dequeue(&self) -> T {
        let mut head_pointer = self.head.lock().unwrap();
        // let start = SystemTime::now();
        let mut head = {
            let mut scoped_head = head_pointer.lock().unwrap();
            loop {
                match *scoped_head {
                    None => {
                        drop(scoped_head);
                        // println!("Waiting for head");
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
        {
            let mut length = self.length.write().unwrap();
            *length = *length - 1;
        }
        return_value
    }
}

pub trait QueuePruneTrait<T> {
    fn prune(&self, value: &HashSet<T>);
}

impl QueuePruneTrait<Board> for Queue<(Option<Board>, Board, usize)> {
    fn prune(&self, list: &HashSet<Board>) {
        let mut _head = self.head.lock().unwrap();
        let mut pseudo_node = Arc::new(Mutex::new(Some(QueueNode { value: vec![], next: _head.clone() })));
        let mut moved_to_node = false;
        loop {
            let node = {
                let mut optional_node = pseudo_node.lock().unwrap();
                if optional_node.is_none() {
                    break;
                }
                // let t = {
                let mut i = 0;
                let node = optional_node.as_mut().unwrap();
                while i < node.value.len() {
                    if let Some(board) = node.value[i].0 {
                        if list.contains(&board) {
                            node.value.remove(i);
                        } else {
                            i += 1;
                        }
                    }
                }
                if !node.value.is_empty() {
                    moved_to_node = true;
                    (true, node.next.clone())
                } else {
                    (false, node.next.clone())
                }
            };
            if node.0 {
                let mut t = pseudo_node.lock().unwrap();
                t.as_mut().unwrap().next = node.1;
            } else {
                pseudo_node = node.1;
            }
        }
    }
}
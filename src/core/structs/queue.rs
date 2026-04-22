use std::{array, cmp, collections::{BTreeSet, HashSet}, fmt::Display, sync::{Arc, Condvar, Mutex, MutexGuard, RwLock}};

use crate::{core::{chess::board::Board, structs::{cash::Cash, concurrent_array_builder::ConcurrentQueuePage}}, log};

pub struct DistributedQueue<T: Cash + Clone + Display, const N: usize> {
    pub size: usize,
    pub queues: Vec<Queue<T, N>>,
}

impl<T: Copy + Cash + Display, const N: usize> DistributedQueue<T, N> {

    pub fn new(size: usize) -> Self {
        let mut queue = DistributedQueue {
            size,
            queues: Vec::with_capacity(size),
        };
        for _ in 0..size {
            queue.queues.push(Queue::new());
        }
        return queue;
    }

    pub fn queue(&self, value: Vec<T>) {
        let mut vectors: Vec<Vec<T>> = Vec::with_capacity(self.size);
        for _ in 0..self.size {
            vectors.push(Vec::with_capacity(value.len()));
        }
        for val in value {
            let index = (val.cash() % self.size as u64) as usize;
            vectors[index].push(val);
        }
        let len = vectors.len();
        for i in 0..len {
            let vec = vectors.pop().unwrap();
            self.queues[len-i-1].queue(vec.iter().as_slice());
        }
        // let current_node = {
        //     let mut current_node = self.current_node.lock().unwrap();
        //     let index_to_queue_to = *current_node;
        //     *current_node = (*current_node + 1) % self.size;
        //     index_to_queue_to
        // };
        // let current_node = {
        //     let mut shortest_queue_length: Option<usize> = None;
        //     let mut shortest_queue_length_index = 0;
        //     for i in 0..self.queues.len() {
        //         let queue_length = {
        //             *(self.queues[i].length.read().unwrap())
        //         };
        //         match shortest_queue_length {
        //             None => {
        //                 shortest_queue_length = Some(queue_length);
        //                 shortest_queue_length_index = i;
        //             }
        //             Some(value) => {
        //                 if queue_length < value {
        //                     shortest_queue_length = Some(queue_length);
        //                     shortest_queue_length_index = i;
        //                 }
        //             }
        //         }
        //     }
        //     shortest_queue_length_index
        // };
        // self.queues[current_node].queue(value);
    }

    // pub fn dequeue(&self, i: usize) -> Vec<T> {
    //     self.queues[i].dequeue()
    // }

    pub fn dequeue_optional<const O: usize>(&self, i: usize, destination: &mut [T; O]) -> usize {
        self.queues[i].dequeue_optional(destination)
    }
}

pub struct QueuePage<T, const N: usize> {
    pub array: ConcurrentQueuePage<T, N>,

    pub next_page: Mutex<Option<Arc<QueuePage<T, N>>>>,
}

impl<T, const N: usize> QueuePage<T, N> {
    fn new() -> Self {
        QueuePage {
            array: ConcurrentQueuePage::new(),
            next_page: Mutex::new(None),
        }
    }
}

pub struct Queue<T, const N: usize> {
    pub head: RwLock<Option<(Arc<QueuePage<T, N>>, usize)>>,
    pub tail: RwLock<Option<(Arc<QueuePage<T, N>>, usize)>>,

    waiter: Arc<Condvar>,
    pub length: Arc<RwLock<usize>>,
}

impl<T: Copy, const N: usize> Queue<T, N> {
    pub fn new() -> Self {
        let q = Queue {
            head: RwLock::new(None),
            tail: RwLock::new(None),

            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        };
        q
    }

    pub fn queue(&self, value: &[T]) {
        if value.is_empty() {
            return;
        }
        {
            let mut l = self.length.write().unwrap();
            *l += value.len();
        }

        let mut index_to_read_from: usize = 0;
        let mut moves_left_to_write = value.len();
        let mut start = None;
        while moves_left_to_write > 0 {
            let mut tail_pointer = self.tail.write().unwrap();
            match tail_pointer.clone() {
                None => {
                    let next_page = (Arc::new(QueuePage::new()), 0);
                    start = Some(next_page.clone());
                    *tail_pointer = Some(next_page);
                },
                Some((page, page_index)) => {
                    if page_index == N {
                        let next_page = (Arc::new(QueuePage::new()), 0);
                        let mut next = page.next_page.lock().unwrap();
                        *next = Some(next_page.0.clone());
                        *tail_pointer = Some(next_page);
                    } else {
                        let space_left = N - page_index;
                        let moves_to_write = cmp::min(space_left, moves_left_to_write);
                        let new_page_index = page_index+moves_to_write;
                        *tail_pointer = Some((page.clone(), new_page_index));
                        drop(tail_pointer);
                        unsafe {
                            // log!("writing: {} {}", page_index, new_page_index);
                            page.array.write(&value[index_to_read_from..index_to_read_from+moves_to_write], page_index, new_page_index);
                        };
                        index_to_read_from += moves_to_write;
                        moves_left_to_write -= moves_to_write;
                    }
                }
            };
        }

        if let Some(pointer) = start {
            let mut head_pointer = self.head.write().unwrap();
            *head_pointer = Some(pointer);
        }
    }

    pub fn dequeue_optional<const O: usize>(&self, destination: &mut [T; O]) -> usize {
        let mut head_pointer = self.head.write().unwrap();
        match head_pointer.clone() {
            None => 0,
            Some((page, page_index)) => {
                let (tail_page, tail_page_index) = { self.tail.read().unwrap().clone().unwrap() };
                let deqeueuer = if Arc::ptr_eq(&page, &tail_page) {
                    if page_index == N {
                        // log!("Here: 1");
                        if let Some(next_page) = page.next_page.lock().unwrap().clone() {
                            *head_pointer = Some((next_page, 0));
                        }
                        None
                    } else {
                        // log!("Here: 2");
                        let moves_to_read = cmp::min(tail_page_index - page_index, O);
                        *head_pointer = Some((page.clone(), page_index + moves_to_read));
                        Some((page, page_index, (page_index + moves_to_read)))
                    }
                } else {
                    if page_index == N {
                        // log!("Here: 3");
                        if let Some(next_page_index) = page.next_page.lock().unwrap().clone() {
                            *head_pointer = Some((next_page_index, 0));
                        }
                        None
                    } else {
                        // log!("Here: 4");
                        let moves_to_read = cmp::min(N - page_index, O);
                        *head_pointer = Some((page.clone(), page_index + moves_to_read));
                        Some((page, page_index, (page_index + moves_to_read)))
                    }
                };
                drop(head_pointer);
                // log!("deqeueuer: {:?}", deqeueuer);
                let l = match deqeueuer {
                    None => 0,
                    Some((page, from, to)) => {
                        unsafe {
                            // log!("Read from: {} to: {}", from, to);
                            page.array.read(destination, from, to);
                        };
                        // // log!("From: {}, To: {}", from, to);
                        to - from
                    }
                };
                // log!("l: {:?}", l);
                {
                    let mut len = self.length.write().unwrap();
                    *len -= l;
                }
                l
            }
        }
    }

    // pub fn dequeue(&self) -> Vec<T> {
    // }

    // pub fn is_empty(&self) -> bool {
    //     self.head.lock().unwrap().lock().unwrap().is_none()
    // }
}
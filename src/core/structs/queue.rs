use std::{array, cmp, collections::HashSet, sync::{Arc, Condvar, Mutex, MutexGuard, RwLock}};

use crate::{core::{chess::board::Board, structs::cash::Cash}, log};

#[derive(Clone)]
pub struct DistributedQueue<T: Cash + Clone> {
    pub size: usize,
    pub queues: Vec<Queue<T>>,
}

impl<T: Copy + Cash> DistributedQueue<T> {

    pub fn new(size: usize) -> Self {
        let mut queue = DistributedQueue {
            size,
            queues: Vec::with_capacity(size),
        };
        for _ in 0..size {
            queue.queues.push(Queue::new(1024));
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
            self.queues[len-i-1].queue(vec);
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

    pub fn dequeue_optional(&self, i: usize) -> Option<Vec<T>> {
        self.queues[i].dequeue_optional()
    }
}

pub struct QueuePage<T> {
    pub array: Vec<T>,
    pub next_page_index: Option<usize>,
}

impl<T> QueuePage<T> {
    fn new(page_size: usize) -> Self {
        QueuePage {
            array: Vec::with_capacity(page_size),
            next_page_index: None,
        }
    }
}

#[derive(Clone)]
pub struct Queue<T> {
    pub page_size: usize,
    pub max_read_size: usize,
    pub head: Arc<Mutex<Option<(usize, usize)>>>,
    pub tail: Arc<Mutex<Option<usize>>>,

    pages: Arc<RwLock<Vec<Arc<Mutex<Option<QueuePage<T>>>>>>>,

    waiter: Arc<Condvar>,
    pub length: Arc<RwLock<usize>>,
}

impl<T: Copy> Queue<T> {
    pub fn new(page_size: usize) -> Self {
        Queue {
            page_size: page_size,
            max_read_size: 20,
            head: Arc::new(Mutex::new(None)),
            tail: Arc::new(Mutex::new(None)),

            pages: Arc::new(RwLock::new(Vec::with_capacity(1024))),

            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        }
    }

    pub fn find_empty_page(self: &Self) -> usize {
        let pages = self.pages.read().unwrap();
        let current_length = { pages.len() };
        for i in 0..current_length {
            let mut page = pages.get(i).unwrap().lock().unwrap();
            if let None = *page {
                *page = Some(QueuePage::new(self.page_size));
                return i;
            }
        }

        drop(pages);
        let mut pages = self.pages.write().unwrap();
        let current_length = { pages.len() };
        for i in 0..current_length {
            let mut page = pages.get(i).unwrap().lock().unwrap();
            if let None = *page {
                *page = Some(QueuePage::new(self.page_size));
                return i;
            }
        }
        pages.push(Arc::new(Mutex::new(Some(QueuePage::new(self.page_size)))));
        current_length
    }

    pub fn queue(&self, value: Vec<T>) {
        if value.is_empty() {
            return;
        }
        {
            let mut l = self.length.write().unwrap();
            *l += value.len();
        }

        let mut index_to_read_from: usize = 0;
        let mut moves_left_to_write = value.len();
        // log!("Queue: Acquiring tail lock");
        // log!("Queue: Value length: {}", value.len());
        let mut tail_pointer = self.tail.lock().unwrap();
        let mut start = None;
        while moves_left_to_write > 0 {
            // log!("Queue: Moves left to write: {}", moves_left_to_write);
            match *tail_pointer {
                None => {
                    // log!("Queue: No end, Finding new empty page");
                    let next_page = self.find_empty_page();
                    start = Some(next_page);
                    *tail_pointer = Some(next_page);
                },
                Some(pages_index) => {
                    // log!("Queue: Reading pages");
                    let pages = self.pages.read().unwrap();
                    // log!("Queue: Unlocking page");
                    let mut page_guard = pages.get(pages_index).unwrap().lock().unwrap();
                    let page = page_guard.as_mut().unwrap();
                    // log!("Queue: len: {}", page.array.len());
                    if page.array.len() == self.page_size {
                        // log!("Queue: Page full, finding new empty page");
                        drop(page_guard);
                        drop(pages);
                        let next_page = self.find_empty_page();
                        // log!("Queue: Reading pages2");
                        let pages = self.pages.read().unwrap();
                        // log!("Queue: Unlocking page2");
                        let mut page = pages.get(pages_index).unwrap().lock().unwrap();
                        let page = page.as_mut().unwrap();
                        page.next_page_index = Some(next_page);
                        *tail_pointer = Some(next_page);
                    } else {
                        let space_left = page.array.capacity() - page.array.len();
                        // log!("Queue: Space left: {}", space_left);
                        let moves_to_write = cmp::min(space_left, moves_left_to_write);
                        // log!("Queue: Moves to write: {}", moves_to_write);
                        for i in 0..moves_to_write {
                            page.array.push(value[index_to_read_from+i]);
                        }
                        index_to_read_from += moves_to_write;
                        moves_left_to_write -= moves_to_write;
                    }
                }
            };
        }
        drop(tail_pointer);

        if let Some(pointer) = start {
            // log!("Queue: Updating head");
            let mut head_pointer = self.head.lock().unwrap();
            *head_pointer = Some((pointer, 0));
        }
    }

    pub fn dequeue_optional(&self) -> Option<Vec<T>> {
        // log!("Dequeue: Acquiring head lock");
        let mut head_pointer = self.head.lock().unwrap();
        match *head_pointer {
            None => None,
            Some((pages_index, array_index)) => {
                // log!("Dequeue: Reading pages");
                let pages = self.pages.read().unwrap();
                // log!("Dequeue: unlocking page");
                let mut option_page = pages.get(pages_index).unwrap().lock().unwrap();
                let page = option_page.as_mut().unwrap();
                // log!("Dequeue: array_index: {}, array_len: {}", array_index, page.array.len());
                if array_index == page.array.len() {
                    // log!("Dequeue: Page empty. Next page: {:?}", page.next_page_index);
                    if let Some(next_pages_index) = page.next_page_index {
                        // log!("Dequeue: Next page available");
                        *option_page = None;
                        *head_pointer = Some((next_pages_index, 0));
                    }
                    None
                } else {
                    let moves_to_read = cmp::min(page.array.len()-array_index, self.max_read_size);
                    // log!("Dequeue: Moves to read: {}", moves_to_read);
                    let mut v = Vec::with_capacity(moves_to_read);
                    for i in 0..moves_to_read {
                        // log!("Dequeue: Adding from index: {}", array_index+i);
                        v.push(page.array[array_index+i])
                    }
                    *head_pointer = Some((pages_index, array_index+moves_to_read));
                    {
                        let mut l = self.length.write().unwrap();
                        *l -= v.len();
                    }
                    Some(v)
                }
            }
        }
    }

    // pub fn dequeue(&self) -> Vec<T> {
    // }

    // pub fn is_empty(&self) -> bool {
    //     self.head.lock().unwrap().lock().unwrap().is_none()
    // }
}
use std::{array, cmp, collections::{BTreeSet, HashSet}, fmt::Display, sync::{Arc, Condvar, Mutex, MutexGuard, RwLock}};

use crate::{core::{chess::board::Board, structs::{cash::Cash, concurrent_array_builder::ConcurrentQueuePage}}, log};

#[derive(Clone)]
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
pub struct Queue<T, const N: usize> {
    pub head: Arc<RwLock<Option<(usize, usize)>>>,
    pub tail: Arc<RwLock<Option<(usize, usize)>>>,

    pages: Arc<RwLock<Vec<Arc<RwLock<Option<(ConcurrentQueuePage<T, N>, Option<usize>)>>>>>>,
    empty_pages: Arc<Mutex<BTreeSet<usize>>>,

    waiter: Arc<Condvar>,
    pub length: Arc<RwLock<usize>>,
}

impl<T: Copy, const N: usize> Queue<T, N> {
    pub fn new() -> Self {
        let size = 1024;
        let mut empty_pages = BTreeSet::new();
        let q = Queue {
            head: Arc::new(RwLock::new(None)),
            tail: Arc::new(RwLock::new(None)),

            pages: Arc::new(RwLock::new(Vec::with_capacity(size))),
            empty_pages: Arc::new(Mutex::new(empty_pages)),

            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        };
        q
    }

    pub fn find_empty_page(self: &Self) -> usize {
        let pages = self.pages.read().unwrap();
        let current_length = { pages.len() };
        let empty_page_index = {self.empty_pages.lock().unwrap().pop_first()};
        if let Some(empty_page_index) = empty_page_index {
            log!("empty_page_index: {}", empty_page_index);
            let page = pages.get(empty_page_index);
            let page = page.unwrap();
            let page = page.read().unwrap();
            if let None = *page {
                drop(page);
                {self.empty_pages.lock().unwrap().remove(&empty_page_index)};
                let mut page = pages.get(empty_page_index).unwrap().write().unwrap();
                if let None = *page {
                    *page = Some((ConcurrentQueuePage::new(), None));
                    return empty_page_index;
                }
            }
        }

        drop(pages);
        let mut pages = self.pages.write().unwrap();
        pages.push(Arc::new(RwLock::new(Some((ConcurrentQueuePage::new(), None)))));
        current_length
    }

    pub fn queue(&self, value: &[T]) {
        // log!("Queueing");
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
            match *tail_pointer {
                None => {
                    let next_page = self.find_empty_page();
                    start = Some(next_page);
                    *tail_pointer = Some((next_page, 0));
                },
                Some((pages_index, page_index)) => {
                    if page_index == N {
                        let next_page_index = self.find_empty_page();

                        let pages = self.pages.read().unwrap();
                        let mut page = pages.get(pages_index).unwrap().write().unwrap();
                        let page = page.as_mut().unwrap();

                        page.1 = Some(next_page_index);
                        *tail_pointer = Some((next_page_index, 0));
                    } else {
                        let space_left = N - page_index;
                        let moves_to_write = cmp::min(space_left, moves_left_to_write);
                        let new_page_index = page_index+moves_to_write;
                        *tail_pointer = Some((pages_index, new_page_index));
                        drop(tail_pointer);
                        let pages = self.pages.read().unwrap();
                        let mut page = pages.get(pages_index).unwrap().read().unwrap();
                        let page = page.as_ref().unwrap();
                        unsafe {
                            // log!("writing: {} {}", page_index, new_page_index);
                            page.0.write(&value[index_to_read_from..index_to_read_from+moves_to_write], page_index, new_page_index);
                        };
                        index_to_read_from += moves_to_write;
                        moves_left_to_write -= moves_to_write;
                    }
                }
            };
        }

        if let Some(pointer) = start {
            let mut head_pointer = self.head.write().unwrap();
            *head_pointer = Some((pointer, 0));
        }
    }

    pub fn dequeue_optional<const O: usize>(&self, destination: &mut [T; O]) -> usize {
        let mut head_pointer = self.head.write().unwrap();
        match *head_pointer {
            None => 0,
            Some((pages_index, page_index)) => {
                let (tail_pages_index, tail_page_index) = { self.tail.read().unwrap().unwrap() };
                let deqeueuer = if pages_index == tail_pages_index {
                    if page_index == N {
                        // log!("Here: 1");
                        let pages = self.pages.read().unwrap();
                        let mut page_container = pages.get(pages_index).unwrap().write().unwrap();
                        let page = page_container.as_mut().unwrap();
                        if let Some(next_page_index) = page.1 {
                            {self.empty_pages.lock().unwrap().insert(pages_index);};
                            *page_container = None;
                            *head_pointer = Some((next_page_index, 0));
                        }
                        None
                    } else {
                        // log!("Here: 2");
                        let moves_to_read = cmp::min(tail_page_index - page_index, O);
                        *head_pointer = Some((pages_index, page_index + moves_to_read));
                        Some((pages_index, page_index, (page_index + moves_to_read)))
                    }
                } else {
                    if page_index == N {
                        // log!("Here: 3");
                        let pages = self.pages.read().unwrap();
                        let mut page_container = pages.get(pages_index).unwrap().write().unwrap();
                        let page = page_container.as_mut().unwrap();
                        if let Some(next_page_index) = page.1 {
                            *page_container = None;
                            *head_pointer = Some((next_page_index, 0));
                        }
                        None
                    } else {
                        // log!("Here: 4");
                        let moves_to_read = cmp::min(N - page_index, O);
                        *head_pointer = Some((pages_index, page_index + moves_to_read));
                        Some((pages_index, page_index, (page_index + moves_to_read)))
                    }
                };
                drop(head_pointer);
                // log!("deqeueuer: {:?}", deqeueuer);
                let l = match deqeueuer {
                    None => 0,
                    Some((pages_index, from, to)) => {
                        let pages = self.pages.read().unwrap();
                        let mut page_container = pages.get(pages_index).unwrap().write().unwrap();
                        let page = page_container.as_mut().unwrap();
                        unsafe {
                            // log!("Read from: {} to: {}", from, to);
                            page.0.read(destination, from, to);
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
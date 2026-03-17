use std::{array, fmt::Display, sync::{Arc, Mutex, RwLock, atomic::{AtomicU64, AtomicUsize}}};

use crate::{core::structs::queue::Queue, log};

#[derive(Clone)]
pub struct ThreadedQueue<T: Display, const N: usize> {
    pub length: Arc<RwLock<usize>>,
    pub thread_count: usize,
    pub queue_index: Arc<AtomicU64>,
    pub dequeue_index: Arc<Mutex<usize>>,
    pub queues: Arc<RwLock<Vec<Queue<T, N>>>>,
}

impl<T: Copy + Display, const N: usize> ThreadedQueue<T, N> {
    pub fn new(thread_count: usize) -> Self {
        let threaded_queue = ThreadedQueue {
            length: Arc::new(RwLock::new(0)),
            thread_count: thread_count,
            queue_index: Arc::new(AtomicU64::new(0)),
            dequeue_index: Arc::new(Mutex::new(0)),
            queues: Arc::new(RwLock::new(Vec::with_capacity(thread_count))),
        };
        for _ in 0..thread_count {
            threaded_queue.queues.write().unwrap().push(Queue::new());
        }
        return threaded_queue;
    }

    pub fn incrment_queue_index(&self) -> u64 {
        self.queue_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn queue(&self, value: Vec<T>) {
        if value.is_empty() {
            return;
        }
        let queue_index = (self.incrment_queue_index() % self.thread_count as u64) as usize;
        let len = value.len();
        // log!("Queueing into index: {:?}, vec size: {:?}", queue_index, value.len());
        *self.length.write().unwrap() += len;
        self.queues.read().unwrap()[queue_index].queue(value.iter().as_slice());
    }

    pub fn dequeue_optional<const O: usize>(&self, destination: &mut [T; N]) -> Option<usize> {
        let mut dequeue_index = self.dequeue_index.lock().unwrap();
        // log!("Queue sizes: {:?}", self.queues.read().unwrap().iter().map(|queue| *queue.length.read().unwrap()).collect::<Vec<usize>>());
        // log!("Dequeueing from index: {:?}", *dequeue_index);
        let res = self.queues.read().unwrap()[*dequeue_index].dequeue_optional(destination);
        if res > 0 {
            Some(res)
        } else {
            None
        }
    }
}
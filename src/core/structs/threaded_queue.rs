use std::{array, sync::{Arc, Mutex, RwLock}};

use crate::{core::structs::queue::Queue, log};

#[derive(Clone)]
pub struct ThreadedQueue<T> {
    pub length: Arc<RwLock<usize>>,
    pub thread_count: usize,
    pub queue_index: Arc<Mutex<usize>>,
    pub dequeue_index: Arc<Mutex<usize>>,
    pub queues: Arc<RwLock<Vec<Queue<T>>>>,
}

impl<T: Clone> ThreadedQueue<T> {
    pub fn new(thread_count: usize) -> Self {
        let threaded_queue = ThreadedQueue {
            length: Arc::new(RwLock::new(0)),
            thread_count: thread_count,
            queue_index: Arc::new(Mutex::new(0)),
            dequeue_index: Arc::new(Mutex::new(0)),
            queues: Arc::new(RwLock::new(Vec::with_capacity(thread_count))),
        };
        for _ in 0..thread_count {
            threaded_queue.queues.write().unwrap().push(Queue::new());
        }
        return threaded_queue;
    }

    pub fn queue(&self, value: Vec<T>) {
        let queue_index = {
            let mut queue_index = self.queue_index.lock().unwrap();
            let index = *queue_index;
            *queue_index = (*queue_index + 1) % self.thread_count;
            index
        };
        let len = value.len();
        // log!("Queueing into index: {:?}, vec size: {:?}", queue_index, value.len());
        *self.length.write().unwrap() += len;
        self.queues.read().unwrap()[queue_index].queue(value);
    }

    pub fn dequeue_optional(&self) -> Option<Vec<T>> {
        let mut dequeue_index = self.dequeue_index.lock().unwrap();
        // log!("Queue sizes: {:?}", self.queues.read().unwrap().iter().map(|queue| *queue.length.read().unwrap()).collect::<Vec<usize>>());
        // log!("Dequeueing from index: {:?}", *dequeue_index);
        let res = self.queues.read().unwrap()[*dequeue_index].dequeue_optional();
        res.map(|value| {
            // log!("Incrementing dequeue index");
            *dequeue_index = (*dequeue_index + 1) % self.thread_count;
            // log!("New dequeue index: {:?}", *dequeue_index);
            let len = value.len();
            *self.length.write().unwrap() -= len;
            value
        })
    }
}
use std::{collections::BTreeMap, sync::{Arc, Mutex, RwLock}};

use crate::core::structs::queue::Queue;

#[derive(Clone)]
pub struct WeightedQueue<T> {
    pub queues: Arc<RwLock<BTreeMap<usize, Queue<T>>>>,
}

impl<T: Clone> WeightedQueue<T> {
    pub fn new() -> Self {
        WeightedQueue {
            queues: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn queue(&self, value: Vec<T>, weight: usize) {
        let mut queues = {
            let readable_queues = self.queues.read().unwrap();
            match readable_queues.get(&weight) {
                Some(queue) => {
                    queue.clone()
                }
                None => {
                    drop(readable_queues);
                    let mut writable_queues = self.queues.write().unwrap();
                    let queue = Queue::new();
                    writable_queues.insert(weight, queue.clone());
                    queue
                }
            }
        };
        queues.queue(value);
    }

    pub fn dequeue_optional(&self) -> Option<(usize, Vec<T>)> {
        // Fetch largest weight queue
        let largest_weight_queue = {
            let readable_queues = self.queues.read().unwrap();
            readable_queues.first_key_value().map(|(key, value)| (*key, value.clone()))
        };
        match largest_weight_queue {
            Some((weight, queue)) => {
                match queue.dequeue_optional() {
                    Some(value) => {
                        Some((weight, value))
                    }
                    None => {
                        drop(queue);
                        let mut writable_queues = self.queues.write().unwrap();
                        let queue = writable_queues.get(&weight);
                        if let Some(queue) = queue {
                            if queue.is_empty() {
                                writable_queues.remove(&weight);
                            }
                        }
                        None
                    }
                }
            }
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        let readable_queues = self.queues.read().unwrap();
        readable_queues.values().map(|queue| *queue.length.read().unwrap()).sum()
    }

    pub fn lengths(&self) -> BTreeMap<usize, usize> {
        let readable_queues = self.queues.read().unwrap();
        readable_queues.iter().map(|(key, queue)| (*key, *queue.length.read().unwrap())).collect()
    }
}

#[derive(Clone)]
pub struct DistributedWeightedQueue<T> {
    pub current_node: Arc<Mutex<usize>>,
    pub size: usize,
    pub queues: Vec<WeightedQueue<T>>,
}

impl<T: Clone> DistributedWeightedQueue<T> {
    pub fn new(size: usize) -> Self {
        DistributedWeightedQueue {
            current_node: Arc::new(Mutex::new(0)),
            size,
            queues: (0..size).map(|_| WeightedQueue::new()).collect(),
        }
    }

    pub fn queue(&self, weight: usize, value: Vec<T>) {
        let current_node = {
            let mut current_node = self.current_node.lock().unwrap();
            let index_to_queue_to = *current_node;
            *current_node = (*current_node + 1) % self.size;
            index_to_queue_to
        };
        self.queues[current_node].queue(value, weight);
    }

    pub fn dequeue_optional(&self, i: usize) -> Option<(usize, Vec<T>)> {
        self.queues[i].dequeue_optional()
    }

    pub fn len(&self) -> usize {
        self.queues.iter().map(|queue| queue.len()).sum()
    }

    pub fn lengths(&self) -> BTreeMap<usize, usize> {
        let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
        for queue in self.queues.iter() {
            for (key, value) in queue.lengths().iter() {
                lengths.insert(*key, lengths.get(key).unwrap_or(&0) + *value);
            }
        }
        lengths
    }
}
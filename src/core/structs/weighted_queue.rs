use std::{collections::BTreeMap, sync::{Arc, Mutex, RwLock}};

use crate::core::structs::{cash::Cash, queue::Queue};

#[derive(Clone)]
pub struct WeightedQueue<T> {
    pub queue_counter: Arc<Mutex<usize>>,
    pub dequeue_counter: Arc<Mutex<usize>>,
    pub queue_size: usize,
    pub queues: [Option<Arc<RwLock<BTreeMap<usize, Queue<T>>>>>; 16],
}

impl<T: Clone> WeightedQueue<T> {
    pub fn new(length: usize) -> Self {
        let mut wq = WeightedQueue {
            queue_counter: Arc::new(Mutex::new(0)),
            dequeue_counter: Arc::new(Mutex::new(0)),
            queue_size: length,
            queues: [const { None } ; 16],
        };
        for i in 0..length {
            wq.queues[i] = Some(Arc::new(RwLock::new(BTreeMap::new())));
        }
        wq
    }

    pub fn queue(&self, value: Vec<T>, weight: usize) {
        let queue_node_index = {
            let mut queue_node_index = self.queue_counter.lock().unwrap();
            let current_queue_node_index = *queue_node_index;
            *queue_node_index = (*queue_node_index + 1) % self.queue_size;
            current_queue_node_index
        };
        let queues = {
            let queue = self.queues.clone()[queue_node_index].clone().unwrap();
            let readable_queue = queue.read().unwrap();
            match readable_queue.get(&weight) {
                Some(queue) => {
                    queue.clone()
                }
                None => {
                    drop(readable_queue);
                    let mut writable_queues = queue.write().unwrap();
                    let queue = Queue::new();
                    writable_queues.insert(weight, queue.clone());
                    queue
                }
            }
        };
        queues.queue(value);
    }

    pub fn dequeue_optional(&self, max: usize) -> Option<(usize, Vec<T>)> {
        let dequeue_node_index = {
            let mut dequeue_node_index = self.dequeue_counter.lock().unwrap();
            let current_dequeue_node_index = *dequeue_node_index;
            *dequeue_node_index = (*dequeue_node_index + 1) % self.queue_size;
            current_dequeue_node_index
        };
        let queues = self.queues.clone()[dequeue_node_index].clone().unwrap();
        // Fetch largest weight queue
        let largest_weight_queue = {
            let readable_queues = queues.read().unwrap();
            readable_queues.first_key_value().map(|(key, value)| (*key, value.clone()))
        };
        match largest_weight_queue {
            Some((weight, queue)) => {
                if weight > max {
                    return None;
                }
                match queue.dequeue_optional() {
                    Some(value) => {
                        Some((weight, value))
                    }
                    None => {
                        drop(queue);
                        let mut writable_queues = queues.write().unwrap();
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

    // pub fn len(&self) -> usize {
    //     let dequeue_node_index = {
    //         let mut dequeue_node_index = self.dequeue_counter.lock().unwrap();
    //         let current_dequeue_node_index = *dequeue_node_index;
    //         *dequeue_node_index = *dequeue_node_index + 1;
    //         current_dequeue_node_index
    //     };
    //     let queues = self.queues.clone()[dequeue_node_index].clone().unwrap();
    //     let readable_queues = queues.read().unwrap();
    //     readable_queues.values().map(|queue| *queue.length.read().unwrap()).sum()
    // }

    pub fn lengths(&self) -> BTreeMap<usize, usize> {
        let mut sum: BTreeMap<usize, usize> = BTreeMap::new();;
        for i in 0..self.queue_size {
            let queues = self.queues.clone()[i].clone().unwrap();
            let readable_queues = queues.read().unwrap();
            for (key, queue) in readable_queues.iter() {
                if !sum.contains_key(key) {
                    sum.insert(*key, 0);
                }
                let curr = sum.get(key).unwrap();
                sum.insert(*key, curr+*queue.length.read().unwrap());
            }
        }
        sum
    }
}

#[derive(Clone)]
pub struct DistributedWeightedQueue<T: Clone + Cash> {
    pub current_node: Arc<Mutex<usize>>,
    pub size: usize,
    pub queues: Arc<RwLock<Vec<WeightedQueue<T>>>>,
}

impl<T: Clone + Cash> DistributedWeightedQueue<T> {
    pub fn new(size: usize) -> Self {
        DistributedWeightedQueue {
            current_node: Arc::new(Mutex::new(0)),
            size,
            queues: Arc::new(RwLock::new((0..size).map(|_| WeightedQueue::new(size)).collect())),
        }
    }

    pub fn queue(&self, weight: usize, value: Vec<T>) {
        let vectors = self.get_vetors(weight, value);
        for i in 0..vectors.len() {
            self.queues.read().unwrap()[i].queue(vectors[i].clone(), weight);
        }

        // let current_node = {
        //     let mut current_node = self.current_node.lock().unwrap();
        //     let index_to_queue_to = *current_node;
        //     *current_node = (*current_node + 1) % self.size;
        //     index_to_queue_to
        // };
        // self.queues.read().unwrap()[current_node].queue(value, weight);
    }

    fn get_vetors(self: &Self, weight: usize, value: Vec<T>) -> Vec<Vec<T>> {
        let mut vectors: Vec<Vec<T>> = Vec::with_capacity(self.size);
        for _ in 0..self.size {
            vectors.push(vec![]);
        }
        for val in value {
            let index = (val.cash() % self.size as u64) as usize;
            vectors[index].push(val);
        }
        vectors
    }

    pub fn dequeue_optional(&self, i: usize, max: usize) -> Option<(usize, Vec<T>)> {
        self.queues.read().unwrap()[i].dequeue_optional(max)
    }

    // pub fn len(&self) -> usize {
    //     self.queues.read().unwrap().iter().map(|queue| queue.len()).sum()
    // }

    pub fn lengths(&self) -> BTreeMap<usize, usize> {
        let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
        for queue in self.queues.read().unwrap().iter() {
            for (key, value) in queue.lengths().iter() {
                lengths.insert(*key, lengths.get(key).unwrap_or(&0) + *value);
            }
        }
        lengths
    }
}
use std::{collections::BTreeMap, fmt::Display, ops::Deref, sync::{Arc, Condvar, Mutex, RwLock}};

use array_builder::ArrayBuilder;

use crate::{core::{chess::board::Board, structs::{cash::Cash, eval_queue::eval_queue::EvalQueue, lock::LockWaiter}}, log};

#[derive(Clone)]
pub struct WeightedEvalQueue<const N: usize> {
    pub thread_count: usize,
    pub queues: Arc<RwLock<BTreeMap<usize, Arc<EvalQueue<N>>>>>,
    waiter: LockWaiter,
    max: Arc<RwLock<usize>>,
}

impl<const N: usize> WeightedEvalQueue<N> {
    pub fn new(thread_count: usize, max: Arc<RwLock<usize>>, waiter: LockWaiter) -> Self {
        WeightedEvalQueue {
            thread_count,
            queues: Arc::new(RwLock::new(BTreeMap::new())),
            waiter: waiter,
            max: max,
        }
    }

    pub fn queue(&self, parent: Board, value: &[Board], weight: usize) {
        let queues = {
            let readable_queues = self.queues.read().unwrap();
            match readable_queues.get(&weight) {
                Some(queue) => {
                    queue.clone()
                }
                None => {
                    drop(readable_queues);
                    let mut writable_queues = self.queues.write().unwrap();
                    let queue = Arc::new(EvalQueue::new());
                    writable_queues.insert(weight, queue.clone());
                    self.waiter.notify();
                    queue
                }
            }
        };
        queues.queue(parent, value.iter().as_slice());
    }

    pub fn dequeue_optional(&self, destination: &mut [Board]) -> Option<(usize, Board, usize)> {
        let mut max = self.max.read().unwrap();
        loop {
            // Fetch largest weight queue
            let lowest_weight_queue = {
                let readable_queues = self.queues.read().unwrap();
                readable_queues.first_key_value().map(|(key, value)| (*key, value.clone()))
            };
            match lowest_weight_queue {
                Some((weight, queue)) => {
                    if weight > *max {
                        drop(max);
                        self.waiter.wait();
                        max = self.max.read().unwrap();
                    } else {
                        let len = queue.dequeue_optional(destination);
                        {
                            if let Some(len) = len && len.1 > 0 {
                                return Some((weight, len.0, len.1))
                            } else {
                                drop(queue);
                                let mut writable_queues = self.queues.write().unwrap();
                                let queue = writable_queues.get(&weight);
                                if let Some(queue) = queue {
                                    if *queue.length.read().unwrap() == 0 {
                                        writable_queues.remove(&weight);
                                    }
                                }
                            }
                        }
                    }
                },
                None => {},
            }
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
pub struct DistributedWeightedEvalQueue<const N: usize> {
    pub size: usize,
    pub queues: Vec<WeightedEvalQueue<N>>,
}

impl<const N: usize> DistributedWeightedEvalQueue<N> {
    pub fn new(size: usize, max: Arc<RwLock<usize>>, waiter: LockWaiter) -> Self {
        DistributedWeightedEvalQueue {
            size,
            queues: (0..size).map(|_| WeightedEvalQueue::new(size, max.clone(), waiter.clone())).collect(),
        }
    }

    pub fn queue(&self, weight: usize, parent: Board, value: &[Board]) {
        let mut thread_indices: ArrayBuilder<usize, 323> = ArrayBuilder::new();
        for val in value.iter() {
            let index = (val.cash() % self.size as u64) as usize;
            thread_indices.push(index);
        }
        for thread_index in 0..self.size {
            let mut vector = ArrayBuilder::<Board, 323>::new();
            for i in 0..value.len() {
                if thread_indices[i] == thread_index {
                    vector.push(value[i]);
                }
            }
            if vector.is_empty() {
                continue;
            }
            self.queues[thread_index].queue(parent, vector.deref(), weight);
        }
        // let current_node = {
        //     let mut current_node = self.current_node.lock().unwrap();
        //     let index_to_queue_to = *current_node;
        //     *current_node = (*current_node + 1) % self.size;
        //     index_to_queue_to
        // };
        // self.queues.read().unwrap()[current_node].queue(value, weight);
    }
    pub fn dequeue_optional(&self, i: usize, destination: &mut [Board]) -> Option<(usize, Board, usize)> {
        self.queues[i].dequeue_optional(destination)
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
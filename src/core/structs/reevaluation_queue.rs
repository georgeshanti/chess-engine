use std::{collections::{BTreeMap, BTreeSet, HashMap}, sync::{Arc, Mutex, RwLock}};

use crate::core::{chess::board::Board};

#[derive(Clone)]
pub struct ReevaluationQueue {
    depth_queues: Arc<RwLock<BTreeMap<usize, Arc<Mutex<BTreeSet<Board>>>>>>,
    board_depth_map: Arc<RwLock<HashMap<Board, usize>>>,
}

impl ReevaluationQueue {

    pub fn new() -> Self {
        ReevaluationQueue {
            depth_queues: Arc::new(RwLock::new(BTreeMap::new())),
            board_depth_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn remove_from_queue(&self, board: Board, depth: usize) {
        let readable_depth_queues = self.depth_queues.read().unwrap();
        if let Some(depth_queue) = readable_depth_queues.get(&depth) {
            let dq = depth_queue.clone();
            let mut dq = dq.lock().unwrap();
            drop(readable_depth_queues);
            dq.remove(&board);
            if dq.is_empty() {
                let mut writable_depth_queues = self.depth_queues.write().unwrap();
                writable_depth_queues.remove(&depth);
            }
        }
    }

    pub fn remove(&self, board: Board) {
        let readable_board_depth_map = self.board_depth_map.read().unwrap();
        let depth = readable_board_depth_map.get(&board);
        if let Some(&depth) = depth {
            drop(readable_board_depth_map);
            let mut writable_board_depth_map = self.board_depth_map.write().unwrap();
            writable_board_depth_map.remove(&board);
            drop(writable_board_depth_map);
            self.remove_from_queue(board, depth);
        }
    }

    fn add_to_queue(&self, board: Board, depth: usize) {
        let mut writable_depth_queues = self.depth_queues.write().unwrap();
        match writable_depth_queues.get(&depth) {
            Some(depth_queue) => {
                let dq = depth_queue.clone();
                let mut dq = dq.lock().unwrap();
                dq.insert(board);
            },
            None => {
                writable_depth_queues.insert(depth, Arc::new(Mutex::new(BTreeSet::from_iter(vec![board]))));
            }
        }
    }

    fn add_to_board_depth_map(&self, board: Board, depth: usize) {
        let mut writable_board_depth_map = self.board_depth_map.write().unwrap();
        let entry = writable_board_depth_map.insert(board, depth);
        match entry {
            Some(current_depth) => {
                if current_depth < depth {
                    writable_board_depth_map.insert(board, depth);
                    drop(writable_board_depth_map);
                    self.add_to_queue(board, depth);
                }
            },
            None => {
                writable_board_depth_map.insert(board, depth);
                drop(writable_board_depth_map);
                self.add_to_queue(board, depth);
            },
        }
    }

    pub fn add(&self, board: Board, depth: usize) {
        let entry = {
            let readable_board_depth_map = self.board_depth_map.read().unwrap();
            readable_board_depth_map.get(&board).map(|&depth| depth)
        };
        match entry {
            None => {
                self.add_to_board_depth_map(board, depth);
            },
            Some(current_depth) => {
                if current_depth < depth {
                    self.add_to_board_depth_map(board, depth);
                }
            }
        }
    }

    pub fn pop(&self) -> Option<(Board, usize)> {
        let highest_depth = {
            let readable_queues = self.depth_queues.read().unwrap();
            readable_queues.last_key_value().map(|entry| (*entry.0, entry.1.clone()))
        };
        highest_depth.and_then(|entry| {
            let depth = entry.0;
            let mut writable_queue = entry.1.lock().unwrap();
            let value = writable_queue.pop_last();
            if let None = value {
                drop(writable_queue);
                let mut writable_queues = self.depth_queues.write().unwrap();
                let queue = writable_queues.get(&depth).map(|queue| queue.clone());
                if let Some(queue) = queue {
                    let queue = queue.lock().unwrap();
                    if queue.is_empty() {
                        writable_queues.remove(&depth);
                    }
                }
            }
            value.map(|value| (value, depth))
        })
    }

    pub fn len(&self) -> usize {
        let readable_queues = self.depth_queues.read().unwrap();
        readable_queues.values().map(|queue| queue.lock().unwrap().len()).sum()
    }
}
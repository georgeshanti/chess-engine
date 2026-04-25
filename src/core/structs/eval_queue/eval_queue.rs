use std::{array, cmp, collections::{BTreeSet, HashSet}, fmt::Display, sync::{Arc, Condvar, LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use crate::{core::{chess::board::Board, structs::{cash::Cash, concurrent_array_builder::ConcurrentQueuePage}}, log};

#[repr(align(64))]
struct CacheLinePaddedRwLock<T> {
    lock: RwLock<T>
}

impl<T> CacheLinePaddedRwLock<T> {
    pub fn new(t: T) -> Self {
        CacheLinePaddedRwLock { lock: RwLock::new(t) }
    }

    pub fn write<'a>(&'a self) -> LockResult<RwLockWriteGuard<'a, T>> {
        self.lock.write()
    }

    pub fn read<'a>(&'a self) -> LockResult<RwLockReadGuard<'a, T>> {
        self.lock.read()
    }
}

pub struct EvalQueuePage<const N: usize> {
    pub array: ConcurrentQueuePage<Board, N>,

    pub next_page: Mutex<Option<Arc<EvalQueuePage<N>>>>,
}

impl<const N: usize> EvalQueuePage<N> {
    fn new() -> Self {
        EvalQueuePage {
            array: ConcurrentQueuePage::new(),
            next_page: Mutex::new(None),
        }
    }
}

pub struct EvalQueue<const N: usize> {
    pub head: CacheLinePaddedRwLock<Option<(Arc<EvalQueuePage<N>>, usize)>>,
    pub tail: CacheLinePaddedRwLock<Option<(Arc<EvalQueuePage<N>>, usize)>>,

    waiter: Arc<Condvar>,
    pub length: Arc<RwLock<usize>>,
}

pub fn tail_pointer_get<'a, T>(m: &'a CacheLinePaddedRwLock<T>) -> RwLockWriteGuard<'a, T> {
    m.write().unwrap()
}

impl<const N: usize> EvalQueue<N> {
    pub fn new() -> Self {
        let q = EvalQueue {
            head: CacheLinePaddedRwLock::new(None),
            tail: CacheLinePaddedRwLock::new(None),

            waiter: Arc::new(Condvar::new()),
            length: Arc::new(RwLock::new(0)),
        };
        q
    }

    pub fn queue(&self, parent: Board, moves: &[Board]) {
        if moves.is_empty() {
            return;
        }
        {
            let mut l = self.length.write().unwrap();
            *l += moves.len();
        }

        let mut index_to_read_from: usize = 0;
        let mut moves_left_to_write = moves.len();
        let mut start = None;
        while moves_left_to_write > 0 {
            let mut tail_pointer = tail_pointer_get(&self.tail);
            match tail_pointer.clone() {
                None => {
                    let next_page = (Arc::new(EvalQueuePage::new()), 0);
                    start = Some(next_page.clone());
                    *tail_pointer = Some(next_page);
                },
                Some((page, page_index)) => {
                    let space_left = N - page_index;
                    if space_left < 3 {
                        let next_page = (Arc::new(EvalQueuePage::new()), 0);
                        let mut next = page.next_page.lock().unwrap();
                        *next = Some(next_page.0.clone());
                        *tail_pointer = Some(next_page);
                    } else {
                        let space_left_for_array = space_left-2;
                        let moves_to_write = cmp::min(space_left_for_array, moves_left_to_write);
                        let array_start = page_index+2;
                        let new_page_index = array_start+moves_to_write;
                        *tail_pointer = Some((page.clone(), new_page_index));
                        drop(tail_pointer);
                        let mut length_board = Board::new();
                        length_board.pieces[0] = moves_to_write as u8;
                        unsafe {
                            page.array.write(&[parent, length_board], page_index, array_start);
                            page.array.write(&moves[index_to_read_from..index_to_read_from+moves_to_write], array_start, new_page_index);
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

    pub fn dequeue_optional(&self, destination: &mut [Board]) -> Option<(Board, usize)> {
        let mut head_pointer = self.head.write().unwrap();
        match head_pointer.clone() {
            None => None,
            Some((page, page_index)) => {
                let (tail_page, tail_page_index) = { self.tail.read().unwrap().clone().unwrap() };
                let end = if Arc::ptr_eq(&page, &tail_page) { tail_page_index } else { N };
                let space_left = end - page_index;
                let deqeueuer = if space_left < 3 {
                    // log!("Here: 3");
                    if let Some(next_page_index) = page.next_page.lock().unwrap().clone() {
                        *head_pointer = Some((next_page_index, 0));
                    }
                    None
                } else {
                    // log!("Here: 4");
                    let array_start = page_index+2;
                    let mut key = [Board::new(), Board::new()];
                    unsafe {page.array.read(&mut key, page_index, array_start);}
                    let [parent, moves_to_read_board] = key;
                    let moves_to_read = moves_to_read_board.pieces[0] as usize;
                    *head_pointer = Some((page.clone(), array_start + moves_to_read));
                    Some((parent, page, page_index + 2, (array_start + moves_to_read)))
                };
                drop(head_pointer);
                // log!("deqeueuer: {:?}", deqeueuer);
                let l = match deqeueuer {
                    None => None,
                    Some((parent, page, from, to)) => {
                        unsafe {
                            // log!("Read from: {} to: {}", from, to);
                            page.array.read(destination, from, to);
                        };
                        // // log!("From: {}, To: {}", from, to);
                        Some((parent, to - from))
                    }
                };
                // log!("l: {:?}", l);
                if let Some((_, l)) = l{
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
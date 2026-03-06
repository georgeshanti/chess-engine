use std::{cell::{Cell, RefCell}, panic, rc::Rc, sync::{Arc, Mutex}};

enum CacheNode<Hot, Cold> {
    Hot(Box<Hot>),
    Cold(Cold),
}

struct LruNode<T> {
    val: T,
    next: Option<usize>,
    prev: Option<usize>,
}

pub struct Lru<T: PartialEq + Copy> {
    head: Option<usize>,
    tail: Option<usize>,
    array: Vec<Option<LruNode<T>>>,
    size: usize,
    capacity: usize,
}

impl<T: PartialEq + Copy> Lru<T> {
    pub fn new() -> Lru<T> {
        Lru {
            head: None,
            tail: None,
            array: Vec::with_capacity(1000),
            size: 0,
            capacity: 1000,
        }
    }

    pub fn add(self: &mut Self, value: T) -> Option<T> {
        let mut index_to_check = self.head;
        let mut found_node = None;
        while let Some(index) = index_to_check {
            if let Some(node) = self.array.get(index).unwrap() {
                if node.val == value {
                    let prev = node.prev;
                    let next = node.prev;
                    if let Some(prev) = prev {
                        if let Some(prev_node) = self.array.get_mut(prev).unwrap() {
                            prev_node.next = next;
                        } else {
                            panic!()
                        }
                    }
                    if let Some(next) = next {
                        if let Some(next_node) = self.array.get_mut(next).unwrap() {
                            next_node.next = prev;
                        } else {
                            panic!()
                        }
                    }
                    if self.head == Some(index) {
                        self.head = next;
                    }
                    if self.tail == Some(index) {
                        self.head = prev;
                    }
                    self.size -= 1;
                    found_node = Some(index);
                    break;
                } else {
                    index_to_check = node.next;
                }
            } else {
                panic!()
            }
        }
        let mut replace = None;
        if self.size == self.capacity - 1 {
            let tail = self.array.get(self.tail.unwrap()).unwrap();
            let tail = tail.as_ref().unwrap();
            replace = Some(tail.val);
            self.tail = tail.prev;
        }
        if let None = found_node {
            let mut index = None;
            for i in 0..self.array.len() {
                if let None = self.array[i] {
                    index = Some(i);
                    break;
                }
            }
            match index {
                Some(index) => {
                    self.array[index] = Some(LruNode { val: value, next: None, prev: None });
                },
                None => {
                    index = Some(self.array.len());
                    self.array.push(Some(LruNode { val: value, next: None, prev: None }));
                }
            }
            let index = index.unwrap();
            found_node = Some(index);
        }
        let found_node = found_node.unwrap();
        let node = self.array.get_mut(found_node).unwrap().as_mut().unwrap();
        node.next = self.head;
        if let Some(head_index) = self.head {
            let head = self.array.get_mut(head_index).unwrap().as_mut().unwrap();
            head.prev = Some(found_node);
        }
        if let None = self.tail {
            self.tail = Some(found_node);
        }
        self.size += 1;
        return replace;
    }

    pub fn remove(self: &mut Self, value: T) {
        let mut index_to_check = self.head;
        while let Some(index) = index_to_check {
            if let Some(node) = self.array.get(index).unwrap() {
                if node.val == value {
                    let prev = node.prev;
                    let next = node.prev;
                    if let Some(prev) = prev {
                        if let Some(prev_node) = self.array.get_mut(prev).unwrap() {
                            prev_node.next = next;
                        } else {
                            panic!()
                        }
                    }
                    if let Some(next) = next {
                        if let Some(next_node) = self.array.get_mut(next).unwrap() {
                            next_node.next = prev;
                        } else {
                            panic!()
                        }
                    }
                    if self.head == Some(index) {
                        self.head = next;
                    }
                    if self.tail == Some(index) {
                        self.head = prev;
                    }
                    self.size -= 1;
                    self.array[index] = None;
                    break;
                } else {
                    index_to_check = node.next;
                }
            } else {
                panic!()
            }
        }
    }
}

pub trait Loader<Hot, Cold> {
    fn load(cold: &Cold) -> Hot;
    fn store(hot: &Hot) -> Cold;
}

pub struct ArrayInternal<Hot, Cold: Clone, L: Loader<Hot, Cold>> {
    loader: L,
    lru: Lru<usize>,
    array: Vec<CacheNode<Hot, Cold>>,
}

impl<Hot, Cold: Clone, L: Loader<Hot, Cold>> ArrayInternal<Hot, Cold, L> {

    pub fn new(loader: L) -> ArrayInternal<Hot, Cold, L> {
        ArrayInternal { loader: loader, lru: Lru::new(), array: Vec::with_capacity(100000) }
    }

    pub fn get(self: &mut Self, index: usize) -> &Hot {
        let array = &mut self.array;
        let either = array.get(index).unwrap();
        if let CacheNode::Cold(cold) = either {
            let hot = L::load(cold);
            array[index] = CacheNode::Hot(Box::new(hot));
            let replace_index = self.lru.add(index);
            if let Some(replace_index) = replace_index {
                if let CacheNode::Hot(hot) = array.get(replace_index).unwrap() {
                    let cold = L::store(hot);
                    array[replace_index] = CacheNode::Cold(cold);
                }
            }
        }
        if let CacheNode::Hot(hot) = array.get(index).unwrap() {
            return hot.as_ref();
        } else {
            panic!()
        }
    }

    pub fn push(self: &mut Self, value: Hot) -> usize {
        let index = self.array.len();
        let array = &mut self.array;
        array.push(CacheNode::Hot(Box::new(value)));
        let replace_index = self.lru.add(index);
        if let Some(replace_index) = replace_index {
            if let CacheNode::Hot(hot) = array.get(replace_index).unwrap() {
                let cold = L::store(hot);
                array[replace_index] = CacheNode::Cold(cold);
            }
        }
        index
    }

    pub fn remove(self: &mut Self, index: usize) {
        let index = self.array.len();
        let array = &mut self.array;
        array.remove(index);
        self.lru.remove(index);
    }
}
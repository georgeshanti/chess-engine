use std::{option, sync::{Arc, Mutex, RwLock}};

struct TreeNode<K, V> {
    value: (K, V),
    left_tree: Arc<Mutex<Option<TreeNode<K, V>>>>,
    right_tree: Arc<Mutex<Option<TreeNode<K, V>>>>,
}

#[derive(Clone)]
pub struct Tree<K, V> {
    root: Arc<Mutex<Option<TreeNode<K, V>>>>,
    length: Arc<RwLock<usize>>,
}

impl<K, V> Tree<K, V> 
where K: PartialEq<K> + PartialOrd<K>, V: Clone {
    pub fn new() -> Self {
        Tree { root: Arc::new(Mutex::new(None)), length: Arc::new(RwLock::new(0)) }
    }

    pub fn len(self: &Self) -> usize {
        let length = self.length.read().unwrap();
        *length
    }

    pub fn compute(self: &Self, key: K, if_present: impl Fn(V), mut if_absent: impl FnMut() -> V) {
        let mut pseudo_node = self.root.clone();
        loop {
            let node = {
                let mut optional_node = pseudo_node.lock().unwrap();
                if optional_node.is_none() {
                    *optional_node = Some(TreeNode { value: (key, if_absent()), left_tree: Arc::new(Mutex::new(None)), right_tree: Arc::new(Mutex::new(None)) });
                    {
                        let mut length = self.length.write().unwrap();
                        *length = *length + 1;
                    }
                    break;
                }
                let node = optional_node.as_ref().unwrap();
                if node.value.0 == key {
                    if_present(node.value.1.clone());
                    break;
                } else if node.value.0 < key {
                    node.right_tree.clone()
                } else {
                    node.left_tree.clone()
                }
            };
            pseudo_node = node;
        };
    }
}
use std::{collections::HashSet, sync::{Arc, Mutex}};

use crate::core::{board::Board, queue::{DistributedQueue}, set::Set};

pub type PositionsToEvaluate = DistributedQueue;
pub type PositionsToReevaluate = Set<Board>;

#[derive(Clone, Copy)]
pub struct PositionToEvaluate {
    pub value: (Option<Board>, Board, usize, i32)
}

impl PositionToEvaluate {
    pub fn get_weight(&self) -> i32 {
        (self.value.2 as i32) - self.value.3
    }
}

impl PartialOrd for PositionToEvaluate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.value.2.cmp(&other.value.2) {
            std::cmp::Ordering::Equal => {
                match self.value.1.cmp(&other.value.1) {
                    std::cmp::Ordering::Equal => self.value.0.partial_cmp(&other.value.0),
                    other => Some(other),
                }
            }
            other => Some(other),
        }
    }
}

impl PartialEq for PositionToEvaluate {
    fn eq(&self, other: &Self) -> bool {
        self.value.0 == other.value.0 && self.value.1 == other.value.1 && self.value.2 == other.value.2
    }
}

impl Eq for PositionToEvaluate {}

impl Ord for PositionToEvaluate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
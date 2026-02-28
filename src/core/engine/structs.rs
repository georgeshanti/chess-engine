use std::time::Instant;

use crate::core::{chess::{board::Board, board_state::Evaluation}, structs::{cash::Cash, queue::DistributedQueue, weighted_queue::DistributedWeightedQueue}};

pub type PositionsToEvaluate = DistributedWeightedQueue<PositionToEvaluate>;
pub type PositionToReevaluate = (Board, (Board, TimestampedEvaluation));
pub type TimestampedEvaluation = (Evaluation, std::time::Instant);
pub type PositionsToReevaluate = DistributedQueue<PositionToReevaluate>;

#[derive(Clone, Copy)]
pub struct PositionToEvaluate {
    pub value: (Option<Board>, Board)
}

impl Cash for PositionToEvaluate {
    fn cash(self: &Self) -> u64 {
        self.value.1.cash()
    }
}

impl Cash for (usize, Board) {
    fn cash(self: &Self) -> u64 {
        self.1.cash()
    }
}

impl Cash for PositionToReevaluate {
    fn cash(self: &Self) -> u64 {
        self.0.cash()
    }
}
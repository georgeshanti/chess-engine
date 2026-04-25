use std::{fmt::{Debug, Display}, time::Instant};

use crate::core::{chess::{board::Board, board_state::Evaluation}, structs::{cash::Cash, eval_queue::distributed_eval_queue::DistributedWeightedEvalQueue, queue::DistributedQueue, weighted_queue::DistributedWeightedQueue}};

pub type PositionsToEvaluate = DistributedWeightedEvalQueue<1024>;
pub type TimestampedEvaluation = (Evaluation, std::time::Instant);
pub type PositionsToReevaluate = DistributedQueue<PositionToReevaluate, 1024>;

#[derive(Clone, Copy)]
pub struct PositionToEvaluate {
    pub value: (Option<Board>, Board)
}

// impl Display for PositionToEvaluate {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "previous:");
//         match self.value.0 {
//             None => {
//                 write!(f, " None");
//             },
//             Some(board) => {
//                 write!(f, "\n");
//                 std::fmt::Display::fmt(&board, f).unwrap();
//             },
//         };
//         write!(f, "\n");
//         std::fmt::Display::fmt(&self.value.1, f).unwrap();
//         Ok(())
//     }
// }

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


#[derive(Clone, Copy)]
pub struct PositionToReevaluate {
    pub value: (Board, (Board, TimestampedEvaluation)),
}

impl Display for PositionToReevaluate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Cash for PositionToReevaluate {
    fn cash(self: &Self) -> u64 {
        self.value.0.cash()
    }
}
use std::{cmp::Ordering, collections::HashSet, fmt::Display, sync::RwLock};

use crate::core::{chess::board::*, engine::structs::TimestampedEvaluation};
use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PositionResult {
    Win,
    Scored,
    Draw,
    Loss,
}

impl Display for PositionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PositionResult::Win => "Win",
            PositionResult::Scored => "Scored",
            PositionResult::Draw => "Draw",
            PositionResult::Loss => "Loss",
        };
        write!(f, "PositionResult(result: {})", s)
    }
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    pub result: PositionResult,
    pub score: i32,
}

impl Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evaluation(result: {}, score: {})", self.result, self.score)
    }
}

impl Evaluation {
    pub fn compare_to(self: &Self, other: &Self) -> Ordering {
        match (self.result, other.result) {
            (PositionResult::Win, PositionResult::Win) => self.score.cmp(&other.score),
            (PositionResult::Win, PositionResult::Scored) => Ordering::Greater,
            (PositionResult::Win, PositionResult::Draw) => Ordering::Greater,
            (PositionResult::Win, PositionResult::Loss) => Ordering::Greater,
            (PositionResult::Scored, PositionResult::Win) => Ordering::Less,
            (PositionResult::Scored, PositionResult::Scored) => self.score.cmp(&other.score),
            (PositionResult::Scored, PositionResult::Draw) => Ordering::Greater,
            (PositionResult::Scored, PositionResult::Loss) => Ordering::Greater,
            (PositionResult::Draw, PositionResult::Win) => Ordering::Less,
            (PositionResult::Draw, PositionResult::Scored) => Ordering::Less,
            (PositionResult::Draw, PositionResult::Draw) => other.score.cmp(&self.score),
            (PositionResult::Draw, PositionResult::Loss) => Ordering::Greater,
            (PositionResult::Loss, PositionResult::Win) => Ordering::Less,
            (PositionResult::Loss, PositionResult::Scored) => Ordering::Less,
            (PositionResult::Loss, PositionResult::Draw) => Ordering::Less,
            (PositionResult::Loss, PositionResult::Loss) => other.score.cmp(&self.score),
        }
    }

    pub fn invert(self: &Self) -> Self {
        match self.result {
            PositionResult::Win => Evaluation{result: PositionResult::Loss, score: self.score+1},
            PositionResult::Scored => Evaluation{result: PositionResult::Scored, score: -self.score},
            PositionResult::Draw => Evaluation{result: PositionResult::Draw, score: self.score},
            PositionResult::Loss => Evaluation{result: PositionResult::Win, score: self.score+1},
        }
    }

    pub fn get_score(&self) -> i32 {
        match self.result {
            PositionResult::Win => 99,
            PositionResult::Scored => self.score,
            PositionResult::Draw => 0,
            PositionResult::Loss => -99,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BoardState {
	pub self_evaluation: Evaluation,
	pub next_moves: Box<[(Board, Option<TimestampedEvaluation>)]>,

	pub previous_moves: RwLock<HashSet<Board>>,

	pub next_best_move: RwLock<Option<NextBestMove>>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NextBestMove {
	pub board: Board,
	pub evaluation: Evaluation,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            self_evaluation: Evaluation{result: PositionResult::Draw, score: 0},
            next_moves: Box::new([]),

            previous_moves: RwLock::new(HashSet::new()),

            next_best_move: RwLock::new(None),
        }
    }
}
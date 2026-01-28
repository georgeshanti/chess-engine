use std::{cmp::Ordering, collections::HashSet, sync::{Arc, RwLock}};

use crate::core::board::*;

#[derive(Clone, Copy, PartialEq)]
pub enum PositionResult {
    Win,
    Scored,
    Draw,
    Loss,
}

#[derive(Copy, Clone, PartialEq)]
pub struct Evaluation {
    pub result: PositionResult,
    pub score: i32,
}

impl Evaluation {
    pub fn compare_to(self: &Self, other: &Self) -> Ordering {
        match (self.result, other.result) {
            (PositionResult::Win, PositionResult::Win) => self.score.cmp(&other.score),
            (PositionResult::Scored, PositionResult::Scored) => self.score.cmp(&other.score),
            (PositionResult::Draw, PositionResult::Draw) => Ordering::Equal,
            (PositionResult::Loss, PositionResult::Loss) => self.score.cmp(&other.score).reverse(),
            (PositionResult::Win, _) => Ordering::Greater,
            (_, PositionResult::Win) => Ordering::Less,
            (PositionResult::Scored, _) => Ordering::Greater,
            (_, PositionResult::Scored) => Ordering::Less,
            (PositionResult::Draw, _) => Ordering::Greater,
            (_, PositionResult::Draw) => Ordering::Less,
            (PositionResult::Loss, _) => Ordering::Less,
            (_, PositionResult::Loss) => Ordering::Greater,
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

pub struct BoardState {
	pub self_evaluation: Evaluation,
	pub next_moves: Box<[Board]>,

	pub previous_moves: RwLock<HashSet<Board>>,

	pub next_best_move: RwLock<Option<NextBestMove>>,
}

#[derive(Clone, Copy, PartialEq)]
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
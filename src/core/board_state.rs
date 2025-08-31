use std::{cmp::Ordering, collections::HashSet, sync::{Arc, RwLock}};

use crate::core::board::*;

#[derive(Clone, Copy)]
pub enum PositionResult {
    Win,
    Scored,
    Draw,
    Loss,
}

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
}

pub struct BoardState {
	pub self_evaluation: Evaluation,
	pub next_moves: Vec<Board>,

	pub previous_moves: RwLock<HashSet<Board>>,

	pub next_best_move: RwLock<Option<NextBestMove>>,
}

pub struct NextBestMove {
	pub board: Board,
	pub evaluation: Evaluation,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            self_evaluation: Evaluation{result: PositionResult::Draw, score: 0},
            next_moves: vec![],

            previous_moves: RwLock::new(HashSet::new()),

            next_best_move: RwLock::new(None),
        }
    }
}
use std::{cmp::Ordering, sync::{RwLock, mpsc::{self, Receiver, Sender}}, thread::sleep, time::{Duration, Instant}};

use crate::{core::{app::App, chess::{board::{Board, BoardArrangement}, board_state::NextBestMove}, engine::structs::{PositionToReevaluate, PositionsToReevaluate}, structs::map::{GroupedPositions, Positions}}, log};
use std::sync::LazyLock;

pub static move_board: LazyLock<RwLock<Board>> = LazyLock::new(|| RwLock::new(Board::new()));
pub static move_board_arrangement: LazyLock<RwLock<BoardArrangement>> = LazyLock::new(|| RwLock::new(BoardArrangement::new()));

pub fn reevaluation_engine(app: App, receiver: Receiver<()>, sender: Sender<()>) {
    let mut handles = vec![];
    let mut wakers: Vec<(Sender<()>, Receiver<()>)> = vec![];
    let app= app.clone();
    for i in 0..app.thread_count {
        let positions_to_reevaluate = app.positions_to_reevaluate.clone();
        let positions = app.positions.clone();
        let (self_tx, self_rx) = mpsc::channel();
        let (thread_tx, thread_rx) = mpsc::channel();
        handles.push(std::thread::Builder::new().name(format!("reevaluation_engine_{}", i)).spawn(move || {
            reevaluation_thread(positions_to_reevaluate, positions, i, thread_rx, self_tx);
        }));
        wakers.push((thread_tx, self_rx));
    }

    loop {
        let _ = receiver.recv().unwrap();
        {
            let app = app.clone();
            *app.status.write().unwrap() = String::from("Re-evaluating positions...");
        }
        for (thread_tx, _) in wakers.iter() {
            thread_tx.send(()).unwrap();
        }
        for (_, self_rx) in wakers.iter() {
            self_rx.recv().unwrap();
        }
        sender.send(()).unwrap();
    }
}

pub fn reevaluation_thread(positions_to_reevaluate: PositionsToReevaluate, positions: GroupedPositions, index: usize, receiver: Receiver<()>, sender: Sender<()>) {
    loop {
        let _ = receiver.recv().unwrap();
        loop {
            let value = {
                let mut count = 0;
                loop {
                    match positions_to_reevaluate.dequeue_optional(index) {
                        Some(value) => {
                            break Some(value);
                        }
                        None => {
                            count += 1;
                            if count > 10 {
                                break None;
                            }
                            // sleep(Duration::from_millis(100));
                        }
                    }
                }
            };

            let value = match value {
                Some(value) => value,
                None => break,
            };

            for (board_to_reevaluate, (next_board, (next_board_new_evaluation, next_board_new_evaluation_timestamp))) in value {
                if let Some(pointer_to_board) = positions.get(&board_to_reevaluate) {
                    let board_arrangement_positions = pointer_to_board.ptr.upgrade();
                    if let Some(board_arrangement_positions) = board_arrangement_positions {
                        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                        let mut board_state = readable_board_arrangement_positions.get(pointer_to_board.index).write().unwrap();
                        let next_moves = &mut board_state.next_moves;

                        let mut should_reevaluate = false;
                        for i in 0..next_moves.len() {
                            let next_position = next_moves[i];
                            if next_position.0 == next_board {
                                if (next_position.1.is_none() || next_position.1.unwrap().1 < next_board_new_evaluation_timestamp) {
                                    next_moves[i].1 = Some((next_board_new_evaluation, next_board_new_evaluation_timestamp));
                                    should_reevaluate = true;
                                }
                                break;
                            }
                        }

                        let mut best_move: Option<NextBestMove> = None;
                        for next_move in next_moves.iter() {
                            match best_move {
                                None => {
                                    if let Some((next_position_evaluation, _)) = next_move.1 {
                                        best_move = Some(NextBestMove{board: next_move.0, evaluation: next_position_evaluation.invert()});
                                    }
                                },
                                Some(present_best_move) => {
                                    if let Some((next_position_evaluation, _)) = next_move.1 {
                                        if next_position_evaluation.compare_to(&present_best_move.evaluation) == Ordering::Less {
                                            best_move = Some(NextBestMove{board: next_move.0, evaluation: next_position_evaluation.invert()});
                                        }
                                    }
                                }
                            }
                        }

                        let mut current_next_best_move = board_state.next_best_move.write().unwrap();
                        if let Some(best_move) = best_move {
                            if current_next_best_move.is_none() || current_next_best_move.unwrap() != best_move {
                                *current_next_best_move = Some(best_move);
                                let queue: Vec<PositionToReevaluate> = board_state.previous_moves.read().unwrap().iter().map(|previous_board| {
                                    (*previous_board, (board_to_reevaluate, (best_move.evaluation, Instant::now())))
                                }).collect();
                                positions_to_reevaluate.queue(queue);
                            }
                        }
                    }
                }
            }
        }
        sender.send(()).unwrap();
    }
}
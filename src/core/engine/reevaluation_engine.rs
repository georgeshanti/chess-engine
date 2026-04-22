use std::{cmp::Ordering, sync::{Arc, RwLock, mpsc::{self, Receiver, Sender}}, thread::sleep, time::{Duration, Instant}};

use crate::{core::{app::App, chess::{board::{Board, BoardArrangement}, board_state::{Evaluation, NextBestMove}}, draw::FixedLengthString, engine::structs::{PositionToReevaluate, PositionsToReevaluate}, structs::map::{GroupedPositions, Positions}}, log};
use std::sync::LazyLock;

pub static move_board: LazyLock<RwLock<Board>> = LazyLock::new(|| RwLock::new(Board::new()));
pub static move_board_arrangement: LazyLock<RwLock<BoardArrangement>> = LazyLock::new(|| RwLock::new(BoardArrangement::new()));

pub fn reevaluation_engine(app: Arc<App>, receiver: Receiver<()>, sender: Sender<()>) {
    let mut handles = vec![];
    let mut wakers: Vec<(Sender<()>, Receiver<()>)> = vec![];
    let app= app.clone();
    for i in 0..app.computer_count {
        let app = app.clone();
        let positions = app.positions.clone();
        let (self_tx, self_rx) = mpsc::channel();
        let (thread_tx, thread_rx) = mpsc::channel();
        handles.push(std::thread::Builder::new().name(format!("reevaluation_engine_{}", i)).spawn(move || {
            reevaluation_thread(app.clone(), positions, i, thread_rx, self_tx);
        }));
        wakers.push((thread_tx, self_rx));
    }

    loop {
        let _ = receiver.recv().unwrap();
        {
            let app = app.clone();
            *app.status.write().unwrap() = FixedLengthString::new(b"Re-evaluating positions...");
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

pub fn reevaluation_thread(app: Arc<App>, positions: GroupedPositions, index: usize, receiver: Receiver<()>, sender: Sender<()>) {
    loop {
        let _ = receiver.recv().unwrap();
        loop {
            let now = Instant::now();
            let mut output: [PositionToReevaluate; 20] = [PositionToReevaluate{value: (Board::new(), (Board::new(), (Evaluation::new(), now)))}; 20];
            let len = app.positions_to_reevaluate.dequeue_optional(index, &mut output);

            if len == 0 {
                break;
            }

            let value = &output[0..len];

            for position_to_reevaluate in value {
                let (board_to_reevaluate, (next_board, (next_board_new_evaluation, next_board_new_evaluation_timestamp))) = position_to_reevaluate.value;
                if board_to_reevaluate == *move_board.read().unwrap() {
                    log!("Checking move: {:?}", next_board.d());
                    log!("Move eval: {}", next_board_new_evaluation);
                }
                if let Some(pointer_to_board) = positions.get(&board_to_reevaluate) {
                    let board_arrangement_positions = pointer_to_board.ptr.upgrade();
                    if let Some(board_arrangement_positions) = board_arrangement_positions {
                        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                        let mut board_state = readable_board_arrangement_positions.get(pointer_to_board.index).write().unwrap();
                        let next_moves = readable_board_arrangement_positions.get_next_moves(board_state.next_moves.0, board_state.next_moves.1, false);

                        let mut should_reevaluate = false;
                        for next_moves in next_moves.clone() {
                            for next_position in next_moves {
                                if next_position.0 == next_board {
                                    let next_position_evaluation = &next_position.1;
                                    let mut next_position_evaluation = next_position_evaluation.write().unwrap();
                                    if (next_position_evaluation.is_none() || next_position_evaluation.unwrap().1 < next_board_new_evaluation_timestamp) {
                                        *next_position_evaluation = Some((next_board_new_evaluation, next_board_new_evaluation_timestamp));
                                        should_reevaluate = true;
                                    }
                                    break;
                                }
                            }
                        }

                        if !should_reevaluate {
                            continue;
                        }

                        let mut best_move: Option<NextBestMove> = None;
                        for next_moves in next_moves {
                            for next_move in next_moves {
                                if let Some((next_position_evaluation, _)) = *next_move.1.read().unwrap() {
                                    let next_position_evaluation_inverted = next_position_evaluation.invert();
                                    match best_move {
                                        None => {
                                            best_move = Some(NextBestMove{board: next_move.0, evaluation: next_position_evaluation_inverted});
                                        },
                                        Some(present_best_move) => {
                                            if next_position_evaluation_inverted.compare_to(&present_best_move.evaluation) == Ordering::Greater {
                                                best_move = Some(NextBestMove{board: next_move.0, evaluation: next_position_evaluation_inverted});
                                            }
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
                                    PositionToReevaluate{value:(*previous_board, (board_to_reevaluate, (best_move.evaluation, Instant::now())))}
                                }).collect();
                                app.positions_to_reevaluate.queue(queue);
                            }
                        }
                    }
                }
            }
        }
        sender.send(()).unwrap();
    }
}
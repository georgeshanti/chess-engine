use std::{cmp::Ordering, sync::{Arc, RwLock}};

use crate::core::{board::Board, board_state::NextBestMove, engine::structs::PositionsToReevaluate, map::Positions, queue::*};



pub fn reevaluation_engine(run_lock: Arc<RwLock<()>>, positions_to_reevaluate: PositionsToReevaluate, positions: Positions) {
    // println!("Re-Evaluation engine started");
    let start_time = std::time::Instant::now();
    // while start_time.elapsed() < RUN_DURATION {
    loop {
        let _unused = run_lock.read().unwrap();
        // println!("Reeval running");
        let board_to_reevaluate = positions_to_reevaluate.dequeue();

        if let Some(pointer_to_board) = positions.get(&board_to_reevaluate) {
            let readable_board_arrangement_positions = pointer_to_board.ptr.upgrade();
            match readable_board_arrangement_positions {
                Some(readable_board_arrangement_positions) => {
                    let readable_board_arrangement_positions = readable_board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
                    let mut next_best_move = board_state.next_best_move.write().unwrap();

                    let mut new_next_best_move: Option<NextBestMove> = None;

                    for next_position in board_state.next_moves.iter() {
                        if let Some(next_position_pointer) = positions.get(&next_position) {
                            match next_position_pointer.ptr.upgrade() {
                                None => continue,
                                Some(board_arrangement_positions) => {
                                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                                    let next_position_board_state = readable_board_arrangement_positions.get(next_position_pointer.index).read().unwrap();
                                    let next_position_best_evaluation =  next_position_board_state.next_best_move.read().unwrap();
                                    let next_position_best_evaluation = match *next_position_best_evaluation {
                                        Some(next_position_best_evaluation) => next_position_best_evaluation.evaluation,
                                        None => next_position_board_state.self_evaluation,
                                    }.invert();
                                    match new_next_best_move {
                                        None => {
                                            new_next_best_move = Some(NextBestMove{
                                                board: *next_position,
                                                evaluation: next_position_best_evaluation,
                                            });
                                        }
                                        Some(current_next_best_move) => {
                                            if current_next_best_move.evaluation.compare_to(&next_position_best_evaluation) == Ordering::Less {
                                                new_next_best_move = Some(NextBestMove{
                                                    board: *next_position,
                                                    evaluation: next_position_best_evaluation,
                                                });
                                            }
                                        }
                                    };
                                }
                            };
                        }
                    }
                    if let Some(new_next_best_move) = new_next_best_move {
                        if Some(new_next_best_move) != *next_best_move {
                            // println!("Updating best move");
                            *next_best_move = Some(new_next_best_move);
        
                            let mut previous_boards: Vec<Board> = Vec::new();
                            for previous_board in board_state.previous_moves.read().unwrap().iter() {
                                previous_boards.push(*previous_board);
                            }
                            positions_to_reevaluate.queue(previous_boards);
                        } else {
                            // println!("Not updating best move #1");
                        }
                    }
                }
                None => {
                    continue;
                }
            }
        }
    }
}
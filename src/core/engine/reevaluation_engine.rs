use std::{cmp::Ordering, collections::HashSet, hash::RandomState, sync::{Arc, RwLock, mpsc::{Receiver, Sender}}, thread::{current, sleep}, time::Duration};

use crate::{core::{board::Board, board_state::NextBestMove, engine::structs::PositionsToReevaluate, map::Positions, queue::*, reevaluation_queue::ReevaluationQueue}, log};



pub fn reevaluation_engine(run_lock: Arc<RwLock<()>>, positions_to_reevaluate: ReevaluationQueue, positions: Positions) {
    // println!("Re-Evaluation engine started");
    let start_time = std::time::Instant::now();
    // while start_time.elapsed() < RUN_DURATION {
    loop {
        let _unused = run_lock.read().unwrap();
        // println!("Reeval running");
        // let value = {
        //     let mut value = 0;
        //     loop {
        //         if value > 5 {
        //             break None;
        //         }
        //         let option = positions_to_reevaluate.pop();
        //         match option {
        //             Some(option) => {
        //                 break Some(option);
        //             }
        //             None => {
        //                 sleep(Duration::from_millis(1));
        //                 continue;
        //             }
        //         }
        //     }
        // };
        let value = positions_to_reevaluate.pop();

        if let None = value {
            break;
        }

        let (board_to_reevaluate, depth) = value.unwrap();

        if let Some(pointer_to_board) = positions.get(&board_to_reevaluate) {
            let board_arrangement_positions = pointer_to_board.ptr.upgrade();
            if let Some(board_arrangement_positions) = board_arrangement_positions {
                let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
                let next_moves = &board_state.next_moves;

                let mut new_next_best_move: Option<NextBestMove> = None;

                for next_position in next_moves.iter() {
                    if let Some(next_position_pointer) = positions.get(&next_position) {
                        if let Some(board_arrangement_positions) =  next_position_pointer.ptr.upgrade() {
                            let next_position_best_evaluation = {
                                let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                                let next_position_board_state = readable_board_arrangement_positions.get(next_position_pointer.index).read().unwrap();
                                let next_position_best_evaluation =  next_position_board_state.next_best_move.read().unwrap();
                                match *next_position_best_evaluation {
                                    Some(next_position_best_evaluation) => next_position_best_evaluation.evaluation.invert(),
                                    None => next_position_board_state.self_evaluation,
                                }
                            };
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
                        };
                    }
                }

                let mut should_reevaluate = false;

                if let Some(new_next_best_move) = new_next_best_move {
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
                    let mut current_next_best_move = board_state.next_best_move.write().unwrap();
                    match *current_next_best_move {
                        Some(ref mut current_next_best_move) => {
                            if new_next_best_move.evaluation.compare_to(&current_next_best_move.evaluation) == Ordering::Greater ||
                                new_next_best_move.board == current_next_best_move.board {
                                *current_next_best_move = new_next_best_move;
                                should_reevaluate = true;
                            }
                        }
                        None => {
                            *current_next_best_move = Some(new_next_best_move);
                            should_reevaluate = true;
                        }
                    }
                }

                if should_reevaluate {

                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
                    let previous_moves = board_state.previous_moves.read().unwrap();
                    for previous_move in previous_moves.iter() {
                        positions_to_reevaluate.add(*previous_move, depth-1);
                    }
                }
            }
        }
    }
}

fn clone_next_moves(next_moves: &Vec<Board>) -> Vec<Board> {
    next_moves.clone()
}

fn clone_previous_moves(previous_moves: &RwLock<HashSet<Board, RandomState>>) -> HashSet<Board> {
    previous_moves.read().unwrap().clone()
}
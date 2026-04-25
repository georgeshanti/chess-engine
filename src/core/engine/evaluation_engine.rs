use std::{collections::HashSet, sync::{Arc, RwLock, mpsc::Sender}, thread::sleep, time::{Duration, Instant}};

use array_builder::ArrayBuilder;
use chrono::{DateTime, Utc};
use ratatui::layout::Position;

use crate::{App, core::{chess::board::{Board, can_come_after_board_arrangement}, engine::{reevaluation_engine::move_board, structs::{PositionToEvaluate, PositionToReevaluate, TimestampedEvaluation}}, structs::map::Presence}, log};
pub static TIMED: RwLock<bool> = RwLock::new(false);

pub fn evaluation_engine(index: usize, app: Arc<App>) {
    let timed: bool = *TIMED.read().unwrap();
    // while time elapsed is less than 10 seconds
    log!("Evaluation engine started");
    let start_time = std::time::Instant::now();
    let positions_to_evaluate = app.positions_to_evaluate.clone();
    let positions = app.positions.clone();
    let positions_to_reevaluate = &app.positions_to_reevaluate;
    loop {
        // sleep(Duration::from_millis(500));
        if timed {
            if start_time.elapsed() > Duration::from_secs(4) {
                log!("Evaluation engine timed out");
                return;
            }
        }
        {
            *(app.thread_stats[index].running_status.write().unwrap()) = false;
        }

        // std::thread::sleep(Duration::from_millis(10000));

        {
            *(app.thread_stats[index].running_status.write().unwrap()) = true;
        }
        // println!("Evaluation engine running");
        // let position = positions_to_evaluate.dequeue(index);
        let mut value = [Board::new(); 323];
        let (board_depth, previous_board, len) = {
            let mut c = 0;
            let res = loop {
                if c > 10 {
                    break None;
                }
                match positions_to_evaluate.dequeue_optional(index, &mut value) {
                    Some(value) => {
                        break Some(value)
                    }
                    None => {
                        // sleep(Duration::from_millis(100));
                        c += 1;
                    }
                }
            };
            match res {
                Some(res) => {res},
                None => continue,
            }
        };
        // log!("board_depth: {}, len: {}", board_depth, len);
        let run_lock_lock = app.run_lock.read().unwrap();
        // if board_depth > 2 {
        //     continue;
        // }
        let current_board_arrangement = {
            app.current_board.read().unwrap()
        }.get_board_arrangement();
        let positions_to_evaluate_list = &value[0..len];
        let mut skippable_set: HashSet<Board> = HashSet::new();
        for board in positions_to_evaluate_list {
            // log!("position got:\n{}", position.value.1);
            // return;
            if Board::new() != previous_board {
                if skippable_set.contains(&previous_board) {
                    continue;
                }
                // if let None = positions.get(&previous_board) {
                if !can_come_after_board_arrangement(&current_board_arrangement, &previous_board.get_board_arrangement()) {
                    skippable_set.insert(previous_board);
                    continue;
                }
            }
            let pointer_to_board = positions.clone().edit(index, &board);
            match pointer_to_board {
                Presence::Absent { value } => {
                    {
                        let mut global_positions_evaluated_count = app.positions_evaluated_acount.write().unwrap();
                        *global_positions_evaluated_count = *global_positions_evaluated_count + 1;
                        let mut positions_evaluated_length = app.thread_stats[index].positions_evaluated_length.write().unwrap();
                        *positions_evaluated_length = *positions_evaluated_length + 1;
                    }
                    let evaluated_board_state = board.get_evaluation();

                    let board_arrangement_positions = value.ptr.upgrade().unwrap();

                    let next_moves = {
                        let mut writable_board_arrangement_positions = board_arrangement_positions.write().unwrap();
                        let mut next_moves: ArrayBuilder<(Board, Option<TimestampedEvaluation>), 323> = ArrayBuilder::new();
                        for board in evaluated_board_state.1.iter() {
                            next_moves.push((*board, None));
                        }
                        let next_moves_size = next_moves.len();
                        let index = writable_board_arrangement_positions.set_next_moves(next_moves.iter().as_slice());
                        (index, next_moves_size)
                    };
    
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let mut writable_board_state = readable_board_arrangement_positions.get(value.index).write().unwrap();
                    writable_board_state.self_evaluation = evaluated_board_state.0;
                    writable_board_state.next_moves = next_moves;
                    if previous_board != Board::new() {
                        {
                            writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                        }
                        {
                            positions_to_reevaluate.queue(vec!(PositionToReevaluate{value: (previous_board, (*board, (evaluated_board_state.0, Instant::now())))}));
                        }
                    }
                    drop(writable_board_state);

                    let next_moves = evaluated_board_state.1.iter();
                    let mut ba_to_send: ArrayBuilder<Board, 40> = ArrayBuilder::new();
                    for next_move in next_moves {
                        ba_to_send.push(*next_move);
                        if ba_to_send.len()==40 {
                            app.positions_to_evaluate.queue(board_depth+1, *board, ba_to_send.iter().as_slice());
                            ba_to_send = ArrayBuilder::new();
                        }
                    }
                    if ba_to_send.len() > 0 {
                        app.positions_to_evaluate.queue(board_depth+1, *board, ba_to_send.iter().as_slice());
                    }
                },
                Presence::Present { value } => {
                    if previous_board != Board::new() {
                        let board_arrangement_positions = value.ptr.upgrade().unwrap();
                        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                        let readable_board_state = readable_board_arrangement_positions.get(value.index).read().unwrap();
                        readable_board_state.previous_moves.write().unwrap().insert(previous_board);
                        {
                            let eval = match *readable_board_state.next_best_move.read().unwrap() {
                                None => {
                                    readable_board_state.self_evaluation
                                },
                                Some(next_best_move) => {
                                    next_best_move.evaluation
                                }
                            };
                            positions_to_reevaluate.queue(vec!(PositionToReevaluate{value: (previous_board, (*board, (eval, Instant::now())))}));
                        }
                    }
                },
            }
        }
        drop(run_lock_lock);
    }
}
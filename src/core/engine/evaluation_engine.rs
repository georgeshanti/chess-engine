use std::{collections::HashSet, sync::{Arc, RwLock, mpsc::Sender}, thread::sleep, time::{Duration, Instant}};

use chrono::{DateTime, Utc};

use crate::{App, core::{chess::board::{Board, can_come_after_board_arrangement}, engine::{reevaluation_engine::move_board, structs::{PositionToEvaluate, TimestampedEvaluation}}, structs::map::Presence}, log};
pub static TIMED: RwLock<bool> = RwLock::new(false);

pub fn evaluation_engine(index: usize, run_lock: Arc<RwLock<()>>, app: App, eval_sender: Sender<(usize, Vec<PositionToEvaluate>)>) {
    let timed: bool = *TIMED.read().unwrap();
    // while time elapsed is less than 10 seconds
    log!("Evaluation engine started");
    let start_time = std::time::Instant::now();
    let positions_to_evaluate = app.positions_to_evaluate.clone();
    let positions = app.positions.clone();
    let positions_to_reevaluate = app.positions_to_reevaluate.clone();
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
        let max_depth = {
            *app.current_depth.read().unwrap()
        };
        let (board_depth, positions_to_evaluate_list) = {
            let mut c = 0;
            let res = loop {
                if c > 10 {
                    break None;
                }
                match positions_to_evaluate.dequeue_optional(index) {
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
        let run_lock_lock = app.run_lock.read().unwrap();
        // if board_depth > 2 {
        //     continue;
        // }
        let current_board_arrangement = {
            app.current_board.read().unwrap().get_board_arrangement()
        };
        let mut skippable_set: HashSet<Board> = HashSet::new();
        for position in positions_to_evaluate_list {
            let (previous_board, board) = position.value;
            if let Some(previous_board) = previous_board {
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
                        let next_moves: Vec<(Board, Option<TimestampedEvaluation>)> = evaluated_board_state.1.clone().iter().map(|board| (board.clone(), None)).collect();
                        let next_moves_size = next_moves.len();
                        let index = writable_board_arrangement_positions.set_next_moves(next_moves);
                        (index, next_moves_size)
                    };
    
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let mut writable_board_state = readable_board_arrangement_positions.get(value.index).write().unwrap();
                    writable_board_state.self_evaluation = evaluated_board_state.0;
                    writable_board_state.next_moves = next_moves;
                    match previous_board {
                        Some(previous_board) => {
                            {
                                writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                            }
                            {
                                positions_to_reevaluate.queue(vec!((previous_board, (board, (evaluated_board_state.0, Instant::now())))));
                            }
                        },
                        _ => {}
                    };
                    drop(writable_board_state);
    
                    let mut next_boards: Vec<PositionToEvaluate> = Vec::with_capacity(evaluated_board_state.1.len());
                    for next_board in evaluated_board_state.1 {
                        next_boards.push(PositionToEvaluate{ value: (Some(board), next_board) });
                    }
                    // positions_to_evaluate.queue(board_depth+1, next_boards);
                    eval_sender.send((board_depth+1, next_boards)).unwrap();
                },
                Presence::Present { value } => {
                    if let Some(previous_board) = previous_board {
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
                            positions_to_reevaluate.queue(vec!((previous_board, (board, (eval, Instant::now())))));
                        }
                    }
                },
            }
        }
        drop(run_lock_lock);
    }
}
use std::{sync::{Arc, RwLock}, thread::sleep, time::Duration};

use crate::{App, core::{engine::structs::{PositionToEvaluate}, structs::map::Presence}, log};
pub static TIMED: RwLock<bool> = RwLock::new(false);

pub fn evaluation_engine(index: usize, run_lock: Arc<RwLock<()>>, app: App) {
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
            if start_time.elapsed() > Duration::from_secs(10) {
                log!("Evaluation engine timed out");
                return;
            }
        }
        {
            *(app.thread_stats[index].running_status.write().unwrap()) = false;
        }

        // std::thread::sleep(Duration::from_millis(10000));
        let _unused = run_lock.read().unwrap();

        {
            *(app.thread_stats[index].running_status.write().unwrap()) = true;
        }
        // println!("Evaluation engine running");
        // let position = positions_to_evaluate.dequeue(index);
        let (board_depth, positions_to_evaluate_list) = {
            let mut c = 0;
            loop {
                if c > 10 {
                    return;
                }
                match positions_to_evaluate.dequeue_optional(index) {
                    Some(value) => {
                        break value
                    }
                    None => {
                        sleep(Duration::from_millis(100));
                        c += 1;
                    }
                }
            }
        };
        // if board_depth > 2 {
        //     continue;
        // }
        for position in positions_to_evaluate_list {
            let (previous_board, board, _, _) = position.value;
            let pointer_to_board = positions.clone().edit(&board);
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
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let mut writable_board_state = readable_board_arrangement_positions.get(value.index).write().unwrap();
                    writable_board_state.self_evaluation = evaluated_board_state.0;
                    writable_board_state.next_moves = evaluated_board_state.1.clone();
                    match previous_board {
                        Some(previous_board) => {
                            {
                                writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                            }
                            {
                                positions_to_reevaluate.queue(vec!((board_depth, board)));
                            }
                        },
                        _ => {}
                    };
                    drop(writable_board_state);
    
                    let mut next_boards: Vec<PositionToEvaluate> = Vec::with_capacity(evaluated_board_state.1.len());
                    for next_board in evaluated_board_state.1 {
                        next_boards.push(PositionToEvaluate{ value: (Some(board), next_board, board_depth + 1, evaluated_board_state.0.get_score()) });
                    }
                    positions_to_evaluate.queue(board_depth+1, next_boards);
                },
                Presence::Present { value } => {
                    if let Some(previous_board) = previous_board {
                        let board_arrangement_positions = value.ptr.upgrade().unwrap();
                        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                        let readable_board_state = readable_board_arrangement_positions.get(value.index).read().unwrap();
                        readable_board_state.previous_moves.write().unwrap().insert(previous_board);
                    }
                },
            }
        }
    }
}
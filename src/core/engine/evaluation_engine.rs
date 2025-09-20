use std::sync::{Arc, RwLock};

use crate::{core::{board::Board, board_state::BoardState, engine::structs::PositionsToReevaluate, map::Presence}, headless, App};

pub fn evaluation_engine(index: usize, run_lock: Arc<RwLock<()>>, app: App) {
    // while time elapsed is less than 10 seconds
    headless!("Evaluation engine started");
    let start_time = std::time::Instant::now();
    // while start_time.elapsed() < RUN_DURATION {
    let positions_to_evaluate = app.positions_to_evaluate.clone();
    let positions = app.positions.clone();
    let positions_to_reevaluate = app.positions_to_reevaluate.clone();
    // loop {
    //     {
    //         *(app.thread_stats[index].running_status.write().unwrap()) = false;
    //     }

    //     std::thread::sleep(Duration::from_millis(2000));
    //     {
    //         *(app.thread_stats[index].running_status.write().unwrap()) = true;
    //     }
    //     std::thread::sleep(Duration::from_millis(2000));
    // }
    loop {
        headless!("Evaluation engine running");
        {
            *(app.thread_stats[index].running_status.write().unwrap()) = false;
        }

        // std::thread::sleep(Duration::from_millis(10000));
        let _unused = run_lock.read().unwrap();

        {
            *(app.thread_stats[index].running_status.write().unwrap()) = true;
        }
        // println!("Evaluation engine running");
        let (previous_board, board, board_depth) = positions_to_evaluate.dequeue(index);
        headless!("Got board");
        // println!("Evaluation engine dequeued: {}", engine_id);
        headless!("Checking if board is present");
        if let Some(previous_board) = previous_board {
            if !positions.clone().is_present(&previous_board) {
                continue;
            }
        }
        let board_state = positions.clone().edit(&board);
        headless!("Got board state");
        match board_state {
            Presence::Absent { value } => {
                headless!("Absent board state");
                {
                    let mut global_positions_evaluated_count = app.positions_evaluated_acount.write().unwrap();
                    *global_positions_evaluated_count = *global_positions_evaluated_count + 1;
                    let mut positions_evaluated_length = app.thread_stats[index].positions_evaluated_length.write().unwrap();
                    *positions_evaluated_length = *positions_evaluated_length + 1;
                }
                let evaluated_board_state = board.get_evaluation();

                let mut writable_board_state = value.write().unwrap();
                headless!("Got writable board state");
                writable_board_state.self_evaluation = evaluated_board_state.0;
                writable_board_state.next_moves = evaluated_board_state.1.clone();
                match previous_board {
                    Some(previous_board) => {
                        writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                        positions_to_reevaluate.add(vec![previous_board]);
                    },
                    _ => {}
                };
                drop(writable_board_state);
                headless!("Dropped writable board state");

                let mut next_boards: Vec<(Option<Board>, Board, usize)> = Vec::new();
                headless!("Inserting next boards");
                if board_depth < 6 {
                    for next_board in evaluated_board_state.1 {
                        // match positions.read().unwrap().get(&next_board) {
                        //     None => {
                        //         next_boards.push((Some(board), next_board, board_depth + 1));
                        //     },
                        //     Some(board_state) => {
                        //         append_parent(board_state, &previous_board, &positions_to_reevaluate);
                        //     }
                        // }
                        next_boards.push((Some(board), next_board, board_depth + 1));
                    }
                    positions_to_evaluate.queue(next_boards);
                }

                // println!("Evaluation engine inserted");
            },
            Presence::Present { value } => {
                headless!("Present board state");
                // println!("Evaluation engine reading");
                append_parent(&value, &previous_board, &positions_to_reevaluate);
            },
        }
        // println!("Evaluation engine completed: {}", engine_id);
    }
    // println!("Evaluation engine completed");
}

fn append_parent(board_state: &Arc<RwLock<BoardState>>, previous_board: &Option<Board>, positions_to_reevaluate: &PositionsToReevaluate) {
    let writable_board_state = board_state.read().unwrap();
    match previous_board {
        Some(previous_board) => {
            let inserted = writable_board_state.previous_moves.write().unwrap().insert(*previous_board);
            if inserted {
                let mut previous_boards: Vec<Board> = Vec::new();
                for previous_board in writable_board_state.previous_moves.read().unwrap().iter() {
                    previous_boards.push(*previous_board);
                }
                positions_to_reevaluate.add(previous_boards);
            }
        },
        _ => {}
    };
}
mod core;

use std::{collections::HashMap, sync::{Arc, Mutex, RwLock}, thread::JoinHandle};

use crate::core::{initial_board::*, board::*, board_state::*, queue::*};

fn evaluation_engine(engine_id: usize, positions: Arc<RwLock<HashMap<Board, Arc<RwLock<BoardState>>>>>, positions_to_evaluate: Arc<Mutex<Queue<(Option<Board>, Board)>>>) {
    // while time elapsed is less than 10 seconds
    println!("Evaluation engine started");
    let start_time = std::time::Instant::now();
    while start_time.elapsed().as_secs() < 10 {
        // println!("Evaluation engine running");
        let (previous_board, board) = positions_to_evaluate.dequeue();
        // println!("Evaluation engine dequeued: {}", engine_id);
        let readable_positions = positions.read().unwrap();
        let board_state = readable_positions.get(&board);
        match board_state {
            None => {
                // println!("Evaluation engine inserting");
                drop(readable_positions);
                let mut writable_positions = positions.write().unwrap();
                let new_board_state = Arc::new(RwLock::new(BoardState::new()));
                writable_positions.insert(board, new_board_state.clone());
                drop(writable_positions);
                let evaluated_board_state = board.get_evaluation();

                let mut writable_board_state = new_board_state.write().unwrap();
                writable_board_state.self_evaluation = evaluated_board_state.0;
                writable_board_state.next_moves = evaluated_board_state.1.clone();
                match previous_board {
                    Some(previous_board) => {
                        writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                    },
                    _ => {}
                };
                drop(writable_board_state);
                for next_board in evaluated_board_state.1 {
                    positions_to_evaluate.queue(vec![(Some(board), next_board)]);
                }
                // println!("Evaluation engine inserted");
            },
            Some(board_state) => {
                // println!("Evaluation engine reading");
                let writable_board_state = board_state.read().unwrap();
                match previous_board {
                    Some(previous_board) => {
                        writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                    },
                    _ => {}
                };
            },
        }
        // println!("Evaluation engine completed: {}", engine_id);
    }
    // println!("Evaluation engine completed");
}

fn main() {
    println!("Hello, world!");
    let positions: Arc<RwLock<HashMap<Board, Arc<RwLock<BoardState>>>>> = Arc::new(RwLock::new(HashMap::new()));
    let positions_to_evaluate: Arc<Mutex<Queue<(Option<Board>, Board)>>> = Arc::new(Mutex::new(Queue::new()));

    positions_to_evaluate.queue(vec![(None, INITIAL_BOARD)]);

    let cpu_count;
    cpu_count = std::thread::available_parallelism().unwrap().get();
    // cpu_count = 1;
    
    let mut threads: Vec<JoinHandle<()>> = Vec::new();
    println!("Starting {} threads", cpu_count);
    for i  in 0..cpu_count {
        let positions = positions.clone();
        let positions_to_evaluate = positions_to_evaluate.clone();
        let join_handle = std::thread::Builder::new().name(format!("evaluation_engine_{}", i)).spawn(move || {
            evaluation_engine(i, positions, positions_to_evaluate);
        }).unwrap();
        threads.push(join_handle);
    }
    let mut i = 1;
    for thread in threads {
        println!("Waiting for thread {}", i);
        thread.join().unwrap();
        println!("Thread {} joined", i);
        i += 1;
    }
    println!("Number of positions: {}", positions.read().unwrap().len());
    println!("All threads joined");
}
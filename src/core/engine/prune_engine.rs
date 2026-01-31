use std::{sync::mpsc::{self, Receiver, Sender}, thread::sleep, time::Duration};

use crate::{App, core::{chess::board::{Board, BoardArrangement, can_come_after_board_arrangement}, engine::reevaluation_engine::move_board_arrangement, structs::map::Positions}, log};

pub fn prune_engine(app: App, next_board: Board) {
    *app.status.write().unwrap() = String::from("Creating vectors...");

    let mut vectors = prune_vectors(app.positions.clone(), app.thread_count-1);

    *app.status.write().unwrap() = String::from("Pruning positions...");

    let mut handles = vec![];
    let (sender, receiver) = mpsc::channel::<BoardArrangement>();
    for i in 0..vectors.len() {
        let board_arrangements = vectors.pop().unwrap();
        let sender = sender.clone();
        handles.push(std::thread::Builder::new().name(format!("prune_engine_checker_{}", i)).spawn(move || {
            prune_thread(next_board, board_arrangements, sender);
        }));
    }

    handles.push(std::thread::Builder::new().name(format!("prune_engine_pruner")).spawn(move || {
        remove_board_arrangements(app.positions.clone(), receiver);
    }));

    drop(sender);
    
    for handle in handles {
        handle.unwrap().join().unwrap();
    }
}

fn prune_vectors(positions: Positions, count: usize) -> Vec<Vec<BoardArrangement>> {
    let board_arrangements: Vec<BoardArrangement> = {
        let writable_positions = positions.map.read().unwrap();
        writable_positions.keys().map(|board_arrangement| board_arrangement.clone()).collect()
    };
    let mut vectors = Vec::with_capacity(count);
    for _ in 0..count {
        vectors.push(Vec::new());
    }
    let mut counter = 0;
    for board_arrangement in board_arrangements {
        vectors[counter].push(board_arrangement);
        counter = (counter + 1) % count;
    }
    return vectors;
}

fn prune_thread(root_board: Board, board_arrangements: Vec<BoardArrangement>, sender: Sender<BoardArrangement>) {
    let current_board_arrangement = root_board.get_board_arrangement();
    for board_arrangement in board_arrangements {
        if !can_come_after_board_arrangement(&current_board_arrangement, &board_arrangement) {
            sender.send(board_arrangement).unwrap();
        }
    }
}

fn remove_board_arrangements(positions: Positions, receiver: Receiver<BoardArrangement>) {
    loop {
        let b = receiver.recv();
        let mut writable_positions = positions.map.write().unwrap();
        match b {
            Ok(b) => {
                writable_positions.remove(&b);
            }
            Err(_) => { break; }
        }
        drop(writable_positions);
        sleep(Duration::from_nanos(1));
    }
}
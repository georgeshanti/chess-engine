use std::{sync::mpsc::{self, Receiver, Sender}, thread::sleep, time::Duration};

use crate::{App, core::{chess::board::{Board, BoardArrangement, can_come_after_board_arrangement}, engine::reevaluation_engine::move_board_arrangement, structs::map::Positions}, log};

pub fn prune_engine(app: App, next_board: Board) {

    *app.status.write().unwrap() = String::from("Pruning positions...");

    let mut handles = vec![];
    for i in 0..app.thread_count {
        let map = app.positions.map[i].clone().unwrap();
        handles.push(std::thread::Builder::new().name(format!("prune_engine_checker_{}", i)).spawn(move || {
            prune_thread(next_board, map);
        }));
    }
    
    for handle in handles {
        handle.unwrap().join().unwrap();
    }
}

fn prune_thread(root_board: Board, positions: Positions) {
    let keys: Vec<BoardArrangement> = {
        positions.map.read().unwrap().keys().map(|f| { f.clone() }).collect()
    };
    let mut writable_map = positions.map.write().unwrap();
    for key in keys {
        if !can_come_after_board_arrangement(&root_board.get_board_arrangement(), &key) {
            writable_map.remove(&key);
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
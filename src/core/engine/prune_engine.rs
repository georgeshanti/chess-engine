// use std::{sync::mpsc::{self, Receiver, Sender}, thread::sleep, time::Duration};

// use crate::{App, core::{chess::board::{Board, BoardArrangement, can_come_after_board_arrangement}, engine::reevaluation_engine::move_board_arrangement, structs::map::Positions}, log};

// pub fn prune_engine(app: App, receiver: Receiver<Board>, sender: Sender<()>) {

//     let mut handles = vec![];
//     let mut wakers: Vec<(Sender<Board>, Receiver<()>)> = vec![];
//     for i in 0..app.computer_count {
//         let map = app.positions.map[i].clone().unwrap();
//         let (self_tx, self_rx) = mpsc::channel();
//         let (thread_tx, thread_rx) = mpsc::channel();
//         handles.push(std::thread::Builder::new().name(format!("prune_engine_checker_{}", i)).spawn(move || {
//             prune_thread(map, thread_rx, self_tx);
//         }));
//         wakers.push((thread_tx, self_rx));
//     }

//     loop {
//         let next_board = receiver.recv().unwrap();
//         log!("Pruning engine started");
//         {
//             let app = app.clone();
//             *app.status.write().unwrap() = String::from("Pruning positions...");
//         }
//         for (thread_tx, _) in wakers.iter() {
//             thread_tx.send(next_board.clone()).unwrap();
//         }
//         for (_, self_rx) in wakers.iter() {
//             self_rx.recv().unwrap();
//         }
//         sender.send(()).unwrap();
//     }
// }

// fn prune_thread(positions: Positions, receiver: Receiver<Board>, sender: Sender<()>) {
//     loop {
//         let root_board = receiver.recv().unwrap();
//         let keys: Vec<BoardArrangement> = {
//             positions.map.read().unwrap().keys().map(|f| { f.clone() }).collect()
//         };
//         for key in keys {
//             let mut writable_map = positions.map.write().unwrap();
//             if !can_come_after_board_arrangement(&root_board.get_board_arrangement(), &key) {
//                 writable_map.remove(&key);
//             }
//             drop(writable_map);
//             // sleep(Duration::from_millis(10));
//         }
//         sender.send(()).unwrap();
//     }
// }

// fn remove_board_arrangements(positions: Positions, receiver: Receiver<BoardArrangement>) {
//     loop {
//         let b = receiver.recv();
//         let mut writable_positions = positions.map.write().unwrap();
//         match b {
//             Ok(b) => {
//                 writable_positions.remove(&b);
//             }
//             Err(_) => { break; }
//         }
//         drop(writable_positions);
//         sleep(Duration::from_nanos(1));
//     }
// }
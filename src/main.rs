mod core;

use crate::core::{app::App, chess::board::{Board, BoardArrangement}, engine::reevaluation_engine::{move_board, move_board_arrangement}, log::FILENAME};

fn main() {
    unsafe {
        let f = format!("logs/{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
        let mut file_name = FILENAME.write().unwrap();
        *file_name = f;

        match std::env::var("LOG") {
            Ok(value) => {
                if value == "false" {
                    let mut enable_log = crate::core::log::ENABLE_LOG.write().unwrap();
                    *enable_log = false;
                }
            },
            Err(_) => {},
        };

        match std::env::var("TIMED") {
            Ok(value) => {
                if value == "true" {
                    let mut timed = crate::core::engine::evaluation_engine::TIMED.write().unwrap();
                    *timed = true;
                }
            },
            Err(_) => {},
        };

        let board: Board = serde_json::from_str("{\"pieces\":[144,0,160,176,168,160,152,144,136,136,136,136,136,136,136,136,0,0,152,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,200,200,0,0,0,0,0,0,0,0,0,0,0,200,200,200,0,0,200,200,200,208,216,224,240,232,224,216,208]}").unwrap();
        *move_board.write().unwrap() = board;

        let board_arrangement: BoardArrangement = serde_json::from_str("{\"higher\":{\"pawns\":402712320,\"major_pieces\":[8,2,2,2,1,1]},\"lower\":{\"pawns\":65280,\"major_pieces\":[8,2,2,2,1,1]}}").unwrap();
        *move_board_arrangement.write().unwrap() = board_arrangement;
    }

    // scratch();
    // return;

    log!("Hello, world!");
    let thread_count = std::thread::available_parallelism().unwrap().get();
    // let thread_count = 6;
    let mut app = App::new(thread_count);

    let _ = app.run();
    ratatui::restore();
}
#![feature(sync_unsafe_cell)]

#[global_allocator]
static GLOBAL: CustomAlloc<System> = CustomAlloc{allocator: System{}};

mod core;

use std::{alloc::System, collections::BTreeMap, io::{Write, stdout}, ops::Deref, os::unix::thread, ptr, sync::{Arc, Mutex, RwLock}, thread::sleep, time::Duration};


use crossterm::{ExecutableCommand, QueueableCommand, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};

use crate::core::{app::App, chess::{board::{Board, BoardArrangement}, board_state::BoardState, initial_board::INITIAL_BOARD, piece::char}, draw::{Block, Borders, Constraint, Direction, Frame, Layout, Margin, convert_usize_to_u8_string}, log::FILENAME, mem::alloc::{CustomAlloc, convert_to_hex, wait}, structs::queue::Queue};

// fn draw(frame: &mut Frame) {
//     frame.render_widget(, frame.area());
// }

fn main() {
    // let t = Arc::new(RwLock::new(5));
    // let t = Arc::new(BTreeMap::<usize, usize>::new());
    // let t = convert_usize_to_u8_string(723);
    // println!("{}", std::str::from_utf8(&t.buf[0..t.length]).unwrap());
    // return;

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

        // let board: Board = serde_json::from_str("{\"pieces\":[144,0,160,176,168,160,152,144,136,136,136,136,136,136,136,136,0,0,152,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,200,200,0,0,0,0,0,0,0,0,0,0,0,200,200,200,0,0,200,200,200,208,216,224,240,232,224,216,208]}").unwrap();
        // *move_board.write().unwrap() = board;
    }

    // let mut stdout = stdout();
    // enable_raw_mode();
    // stdout.execute(EnterAlternateScreen);

    // let mut app = App::new(14, 2);
    // let mut frame = Frame::new();
    // app.draw(&mut frame);
    // frame.stdout.flush();
    // sleep(Duration::from_secs(5));
    // return;
    // // std::thread::sleep(Duration::from_millis(5000));
    // stdout.execute(LeaveAlternateScreen);
    // disable_raw_mode();
    // return;


    // let init = [INITIAL_BOARD; 1];
    // let q: Queue<Board, 10> = Queue::new();
    // q.queue(&init);
    // let mut init: [Board; 10] = [Board::new(); 10];
    // let len = q.dequeue_optional(&mut init);
    // println!("Len: {}\nBoard:\n{}\n", len, init[0]);
    // return;


    // scratch();
    // return;
    let mut app = App::new(14, 2);
    println!("Done");
    let _ = app.run();
    // ratatui::restore();
}
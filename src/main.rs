mod core;

use std::{sync::{Arc, Mutex, RwLock, mpsc::{self, Receiver, Sender}}, thread::{JoinHandle, sleep}, time::Duration};
use ratatui::{crossterm::event::{read, poll, Event, KeyCode}, layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, widgets::{Block, Borders, Paragraph}, DefaultTerminal, Frame};
use regex::Regex;
use thousands::Separable;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::core::{app::{App, ThreadStat}, chess::{board::*, initial_board::INITIAL_BOARD}, engine::{evaluation_engine::*, prune_engine::prune_engine, reevaluation_engine::*, structs::{PositionToEvaluate, PositionsToEvaluate, PositionsToReevaluate}}, log::{ENABLE_LOG, FILENAME}, structs::{map::{PAGE_BOARD_COUNT, Positions}, queue::*, reevaluation_queue::ReevaluationQueue, set::Set, weighted_queue::{DistributedWeightedQueue, WeightedQueue}}};

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
    }

    // scratch();
    // return;

    log!("Hello, world!");
    let thread_count = std::thread::available_parallelism().unwrap().get();
    let (tx, rx): (Sender<Board>, Receiver<Board>) = mpsc::channel();
    // let thread_count = 2;
    let mut app = App {
        positions: Positions::new(),
        positions_to_evaluate: DistributedWeightedQueue::new(thread_count),
        positions_to_reevaluate: DistributedQueue::new(thread_count),
        run_lock:  Arc::new(RwLock::new(())),
        current_board: Arc::new(Mutex::new(INITIAL_BOARD)),
        thread_stats: Vec::with_capacity(thread_count),
        thread_count: thread_count,
        positions_evaluated_acount: Arc::new(RwLock::new(0)),
        frame_count: 0,
        input: Arc::new(RwLock::new(Input::new(String::from("")))),
        editing: Arc::new(RwLock::new(true)),
        prompt: String::from("Enter move:"),
        start_time: std::time::Instant::now(),
        status: Arc::new(RwLock::new(String::from("Evaluating..."))),
    };

    for _ in 0..thread_count {
        app.thread_stats.push(ThreadStat::new());
    }

    let result = app.run(rx);
    ratatui::restore();
}
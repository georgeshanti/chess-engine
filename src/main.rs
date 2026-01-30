mod core;

use std::{sync::{Arc, Mutex, RwLock, mpsc::{self, Receiver, Sender}}, thread::{JoinHandle, sleep}, time::Duration};
use humantime::format_duration;
use ratatui::{crossterm::event::{read, poll, Event, KeyCode}, layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, widgets::{Block, Borders, Paragraph}, DefaultTerminal, Frame};
use regex::Regex;
use thousands::Separable;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::core::{board::{self, *}, board_state::BoardState, engine::{evaluation_engine::*, reevaluation_engine::*, structs::{PositionToEvaluate, PositionsToEvaluate, PositionsToReevaluate}}, initial_board::*, log::{ENABLE_LOG, FILENAME}, map::{PAGE_BOARD_COUNT, Positions}, piece::*, queue::*, reevaluation_queue::ReevaluationQueue, set::Set, weighted_queue::DistributedWeightedQueue};

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

fn prune_engine(root_board: Board, board_arrangements: Vec<BoardArrangement>, sender: Sender<BoardArrangement>) {
    let current_board_arrangement = root_board.get_board_arrangement();
    for board_arrangement in board_arrangements {
        if !can_come_after_board_arrangement(&board_arrangement, &current_board_arrangement) {
            sender.send(board_arrangement);
        }
    }
    log!("Ended pruning");
}

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

    // println!("{}", PAGE_BOARD_COUNT);
}

#[derive(Clone)]
struct App {
    current_board: Arc<Mutex<Board>>,
    positions: Positions,
    positions_to_evaluate: PositionsToEvaluate,
    positions_to_reevaluate: PositionsToReevaluate,
    run_lock: Arc<RwLock<()>>,
    thread_stats: Vec<ThreadStat>,
    thread_count: usize,
    positions_evaluated_acount: Arc<RwLock<usize>>,
    frame_count: usize,
    input: Arc<RwLock<Input>>,
    editing: Arc<RwLock<bool>>,
    prompt: String,
    start_time: std::time::Instant,
    status: Arc<RwLock<String>>,
}

#[derive(Clone)]
struct ThreadStat{
    positions_evaluated_length: Arc<RwLock<usize>>,
    running_status: Arc<RwLock<bool>>,
}

impl ThreadStat {
    fn new() -> Self {
        ThreadStat {
            positions_evaluated_length: Arc::new(RwLock::new(0)),
            running_status: Arc::new(RwLock::new(false)),
        }
    }
}

impl App {
    fn run(&mut self, receiver_positions_to_reevaluate: Receiver<Board>) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.clone();
        let _unused = std::thread::Builder::new().name(format!("app_main")).spawn(move || {
            app.run_engine(app.thread_stats.len(), receiver_positions_to_reevaluate);
        }).unwrap();
        {
            let mut t = self.clone();
            let _used = std::thread::Builder::new().name(String::from("Drawer")).spawn(move || {
                let mut terminal = ratatui::init();
                loop {
                    terminal.draw(|frame| t.draw(frame)).unwrap();
                    std::thread::sleep(Duration::from_millis(100));
                }
            }).unwrap();
        }
        loop {
            
            if poll(Duration::from_millis(100))? {
                log!("Got event");
                let event = read().unwrap();
                let mut editing = self.editing.write().unwrap();
                if *editing {
                    log!("Editing");
                    match event {
                        Event::Key(key_event) => {
                            log!("Got key event");
                            if key_event.code == KeyCode::Esc {
                                log!("Got Esc key event");
                                *editing = false;
                            } else if key_event.code == KeyCode::Enter {
                                log!("Got Enter key event");
                                *editing = false;
                                log!("Processing prompt");
                                drop(editing);
                                self.process_prompt();
                            } else {
                                log!("Forward to input");
                                self.input.write().unwrap().handle_event(&event);
                            }
                        },
                        _ => {},
                    };
                } else {
                    log!("Not editing");
                    match event {
                        Event::Key(key_event) => {
                            log!("Got key event");
                            if key_event.code == KeyCode::Enter {
                                log!("Got Enter key event. Entering edit mode.");
                                *editing = true;
                            } else if key_event.code == KeyCode::Esc {
                                log!("Got Esc key event. Exiting application.");
                                break Ok(());
                            }
                        },
                        _ => {},
                    };
                }
            } else {
                // log!("Timeout expired. No event.");
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        // let _unused = self.run_lock.write().unwrap();
        let thread_count = self.thread_stats.len();
        let [left_pane, right_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(frame.area()).as_ref().try_into().unwrap();
        let [global_status_pane, thread_status_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(5), Constraint::Fill(1)])
            .split(right_pane.inner(Margin::new(1, 1))).as_ref().try_into().unwrap();

        let [board_pane, prompt_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(3),
            ])
            .split(left_pane).as_ref().try_into().unwrap();

        let status_pane = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4); thread_count])
            .split(thread_status_pane.inner(Margin::new(1, 1)));

        frame.render_widget(Block::default().borders(Borders::ALL), board_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), right_pane);
        
        for i in 0..self.thread_stats.len() {
            App::draw_stat(frame, i, &self.thread_stats[i], status_pane[i]);
        }

        frame.render_widget(Paragraph::new(format!("{}", self.current_board.lock().unwrap())), board_pane.inner(Margin::new(1, 1)));
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(Paragraph::new(self.prompt.clone()), prompt_pane.inner(Margin::new(1, 0)));
        frame.render_widget(Paragraph::new(format!("{}", self.input.read().unwrap().value())), prompt_pane.inner(Margin::new(1, 1)));

        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[0]);

        let [eval_queue_stat_pane, reval_queue_stat_pane, board_pieces_pane, positions_evaluated_pane, positions_evaluated_pseudo_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .split(global_status_pane).as_ref().try_into().unwrap();

        let [eval_queue_stat_name_pane, eval_queue_stat_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
            .split(eval_queue_stat_pane).as_ref().try_into().unwrap();

        let [reval_queue_stat_name_pane, reval_queue_stat_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
            .split(reval_queue_stat_pane).as_ref().try_into().unwrap();
        let [board_pieces_name_pane, board_pieces_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50); 2])
            .split(board_pieces_pane).as_ref().try_into().unwrap();
        let [positions_evaluated_name_pane, positions_evaluated_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50); 2])
            .split(positions_evaluated_pane).as_ref().try_into().unwrap();
        let [positions_evaluated_pseudo_name_pane, positions_evaluated_pseudo_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50); 2])
            .split(positions_evaluated_pseudo_pane).as_ref().try_into().unwrap();

        frame.render_widget(Paragraph::new("Reval Queue:"), reval_queue_stat_pane);
        let mut length = 0;
        for i in 0..self.thread_stats.len() {
            let l = self.positions_to_reevaluate.queues[i].length.read().unwrap();
            length += *l;
        }
        frame.render_widget(Paragraph::new(format!("{}", length.separate_with_commas())).alignment(Alignment::Right), reval_queue_stat_value_pane);

        frame.render_widget(Paragraph::new("Eval Queue:"), eval_queue_stat_name_pane);
        let lengths = self.positions_to_evaluate.lengths();
        let mut lengths_string = String::from("{");
        for length in lengths.iter() {
            lengths_string += &format!("{}: {}, ", length.0, length.1.separate_with_commas());
        }
        lengths_string += "}";
        frame.render_widget(Paragraph::new(lengths_string).alignment(Alignment::Right), eval_queue_stat_value_pane);

        frame.render_widget(Paragraph::new("Board Pieces:"), board_pieces_name_pane);
        let len = {
            self.positions.map.read().unwrap().len()
        };
        frame.render_widget(Paragraph::new(format!("{}", len.separate_with_commas())).alignment(Alignment::Right), board_pieces_value_pane);
        frame.render_widget(Paragraph::new("Positions evaluated:"), positions_evaluated_name_pane);
        frame.render_widget(Paragraph::new(format!("{}", self.positions.len().separate_with_commas())).alignment(Alignment::Right), positions_evaluated_value_pane);
        frame.render_widget(Paragraph::new("Positions evaluated pseudo:"), positions_evaluated_pseudo_name_pane);
        frame.render_widget(Paragraph::new(format!("{}", self.positions_evaluated_acount.read().unwrap().separate_with_commas())).alignment(Alignment::Right), positions_evaluated_pseudo_value_pane);
        frame.render_widget(Paragraph::new(format!("Time: {:?} Engine status: {}", self.start_time.elapsed().as_secs(), self.status.read().unwrap())), right_pane);
        self.frame_count = self.frame_count + 1;
    }

    fn draw_stat(frame: &mut Frame, index: usize,thread_stat: &ThreadStat, rect: Rect) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50); 2])
            .split(rect.inner(Margin::new(1, 1)));
        let left_bars = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); 2])
            .split(panes[0]);
        let right_bars = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); 2])
            .split(panes[1]);
        frame.render_widget(Block::default().borders(Borders::ALL), rect);
        frame.render_widget(Paragraph::new(format!("Thread #{}", index)), rect.inner(Margin::new(1, 0)));
        frame.render_widget(Paragraph::new(format!("{}", thread_stat.running_status.read().unwrap())).alignment(Alignment::Right), right_bars[0]);
        frame.render_widget(Paragraph::new("Positions evaluated"), left_bars[1]);
        frame.render_widget(Paragraph::new(format!("{}", thread_stat.positions_evaluated_length.read().unwrap().separate_with_commas())).alignment(Alignment::Right), right_bars[1]);
    }

    fn process_prompt(&mut self) {
            let mut current_board = self.current_board.lock().unwrap();
            let re = Regex::new(r"([a-z])(\d)-([a-z])(\d)").unwrap();
            let mut input = self.input.write().unwrap();
            let captures = match re.captures(input.value()){
                Some(captures) => captures,
                None => {
                    self.prompt = String::from("Invalid syntax. Enter move:");
                    input.reset();
                    return;
                }
            };
            let from_file = ((captures[1].chars().nth(0).unwrap() as u32) - 'a' as u32) as usize;
            let from_rank = captures[2].parse::<usize>().unwrap()-1;
            let to_file = ((captures[3].chars().nth(0).unwrap() as u32) - 'a' as u32) as usize;
            let to_rank = captures[4].parse::<usize>().unwrap()-1;
            let source_piece = current_board.get(from_rank, from_file);
            let target_piece = current_board.get(to_rank, to_file);

            log!("Processing prompt: from_file: {}, from_rank: {}, to_file: {}, to_rank: {}", from_file, from_rank, to_file, to_rank);

            if get_presence(source_piece) == EMPTY || get_color(source_piece) == BLACK || !(get_presence(target_piece) == EMPTY || get_color(target_piece) == BLACK) {
                log!("{}, {}, {}, {}. Syntax error.", get_presence(source_piece) == EMPTY, get_color(source_piece) == BLACK, get_presence(target_piece) == EMPTY, get_color(target_piece) == BLACK);
                self.prompt = String::from("Invalid move. Enter move:");
                input.reset();
                return;
            }

            log!("Processing prompt: Valid pieces present in source and target squares");
            log!("Processing prompt: current_board: {:?} \n{}", current_board.pieces, current_board);
            let next_board = {
                let current_board_state = self.positions.get(&*current_board);
                if let Some(pointer_to_board) = current_board_state {
                    log!("Processing prompt: Found board state for current position");

                    let board_arrangement_positions = pointer_to_board.ptr.upgrade().unwrap();
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
    
                    let mut next_board: Option<Board> = None;
                    for next_move in board_state.next_moves.iter() {
                        let source_piece = next_move.get(7-from_rank, 7-from_file);
                        let target_piece = next_move.get(7-to_rank, 7-to_file);
                        log!("Processing prompt: candidate:\n{}", next_move.inverted());
                        if get_presence(source_piece) == EMPTY && get_presence(target_piece) == PRESENT && get_color(target_piece) == BLACK {
                            next_board = Some(*next_move);
                            break;
                        }
                    }
                    match next_board {
                        Some(next_board) => next_board,
                        None => {
                            log!("Processing prompt: Could not find move corresponding to prompt");
                            self.prompt = String::from("Invalid move 2. Enter move:");
                            input.reset();
                            return;
                        }
                    }
                } else {
                    log!("Processing prompt: Could not find board state for current position");
                    self.prompt = String::from("Invalid move 3. Enter move:");
                    input.reset();
                    return;
                }
            };
            let editing = self.editing.clone();
            let app = self.clone();
            std::thread::Builder::new().name(format!("reevaluation_engine_main")).spawn(move || {
                {
                    let app = app.clone();
                    let mut handles = vec![];
                    {
                        *app.status.write().unwrap() = String::from("Creating vectors...");
                    }
                    let mut vectors = prune_vectors(app.positions.clone(), app.thread_count-1);
                    {
                        *app.status.write().unwrap() = String::from("Pruning positions...");
                    }
                    let (sender, receiver) = mpsc::channel::<BoardArrangement>();
                    for i in 0..app.thread_count-1 {
                        let board_arrangements = vectors.pop().unwrap();
                        let current_board = app.current_board.lock().unwrap().clone();
                        let sender = sender.clone();
                        log!("Spawning prune_engine_{}", board_arrangements.len());
                        handles.push(std::thread::Builder::new().name(format!("prune_engine_{}", i)).spawn(move || {
                            prune_engine(current_board, board_arrangements, sender);
                        }));
                    }
                    handles.push(std::thread::Builder::new().name(format!("send_board_arrangements")).spawn(move || {
                        remove_board_arrangements(app.positions.clone(), receiver);
                    }));
                    drop(sender);
                    for handle in handles {
                        handle.unwrap().join().unwrap();
                    }
                }

                // {
                //     {
                //         *app.status.write().unwrap() = String::from("Re-evaluating positions...");
                //     }
                //     let mut handles = vec![];
                //     for i in 0..app.thread_count {
                //         let run_lock = app.run_lock.clone();
                //         let positions_to_reevaluate = app.positions_to_reevaluate.clone();
                //         let positions = app.positions.clone();
                //         handles.push(std::thread::Builder::new().name(format!("reevaluation_engine_{}", i)).spawn(move || {
                //             reevaluation_engine(run_lock, positions_to_reevaluate, positions, i);
                //         }));
                //     }
                //     for handle in handles {
                //         handle.unwrap().join().unwrap();
                //     }
                //     {
                //         *app.status.write().unwrap() = String::from("Evaluating...");
                //     }
                // }
                let mut editing = editing.write().unwrap();
                *editing = true;
            }).unwrap();
            
            // {
            //     let next_board_state = self.positions.get(&next_board);
            //     match next_board_state {
            //         Some(pointer_to_board) => {

            //             let board_arrangement_positions = pointer_to_board.ptr.upgrade().unwrap();
            //             let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
            //             let next_board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
            //             let next_best_move = next_board_state.next_best_move.read().unwrap();
            //             match *next_best_move {
            //                 None => {
            //                     log!("Processing prompt: No next best move found for entered move's position");
            //                     self.prompt = String::from("Have not evaluated position yet. Enter move:");
            //                     input.reset();
            //                     return;
            //                 }
            //                 Some(next_best_move) => {
            //                     log!("Processing prompt: Setting current board to {}", next_best_move.board);
            //                     log!("Setting current board to {}", next_best_move.board);
            //                     *current_board = next_best_move.board;
            //                     input.reset();
            //                 }
            //             }
            //         },
            //         None => {
            //             // println!("Positions: {}", positions.len());
            //             // println!("Depth: {}", DEPTH.lock().unwrap());
            //             log!("Processing prompt: Could not find board state for entered move's position");
            //             self.prompt = String::from("Have not evaluated position yet. Enter move:");
            //             input.reset();
            //             return;
            //         }
            //     }
            // }
    
            // prune_engine(self.run_lock.clone(), self.positions.clone(), self.positions_to_evaluate.clone(), *current_board);
    }

    fn run_engine(&self, thread_count: usize, receiver_positions_to_reevaluate: Receiver<Board>) {
        log!("Running engine");
        self.positions_to_evaluate.queue(0, vec![PositionToEvaluate{ value: (None, INITIAL_BOARD, 0, 0) }]);
        log!("queued");
        let mut threads: Vec<JoinHandle<()>> = Vec::new();
        log!("Starting {} threads", thread_count);
        for i  in 0..self.thread_stats.len() {
            let app = self.clone();
            let run_lock = self.run_lock.clone();
            let join_handle = std::thread::Builder::new().name(format!("evaluation_engine_{}", i)).spawn(move || {
                evaluation_engine(i, run_lock, app);
            }).unwrap();
            threads.push(join_handle);
        }
        log!("threads started");
        // {
        //     let positions = self.positions.clone();
        //     let positions_to_reevaluate = self.positions_to_reevaluate.clone();
        //     let run_lock = self.run_lock.clone();
        //     let _unused = std::thread::Builder::new().name(format!("reevaluation_engine")).spawn(move || {
        //         reevaluation_engine(run_lock, positions_to_reevaluate, positions);
        //     }).unwrap();
        // }

        // for m in current_board.find_moves() {
        //     println!("{}", m.inverted());
        // }
        // return;
        loop {}
    }
}

fn scratch() {
    let board = Board{pieces: [144, 152, 160, 168, 176, 160, 152, 144, 136, 136, 136, 0, 0, 136, 136, 136, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136, 216, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 200, 200, 200, 200, 200, 200, 200, 200, 208, 216, 224, 232, 240, 224, 0, 208]};
    let t = board.get_evaluation();
    for n in t.1.iter()  {
        println!("{}", n.inverted());
    }
}

fn remove_board_arrangements(positions: Positions, receiver: Receiver<BoardArrangement>) {
    let mut writable_positions = positions.map.write().unwrap();
    loop {
        let b = receiver.recv();
        match b {
            Ok(b) => {
                writable_positions.remove(&b);
            }
            Err(e) => { break; }
        }
        // drop(writable_positions);
        // sleep(Duration::from_millis(10));
    }
}
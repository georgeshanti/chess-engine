mod core;

use std::{sync::{Arc, Mutex, RwLock}, thread::JoinHandle, time::Duration};
use ratatui::{crossterm::event::{read, poll, Event, KeyCode}, layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, widgets::{Block, Borders, Paragraph}, DefaultTerminal, Frame};
use regex::Regex;
use thousands::Separable;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::core::{board::*, engine::{evaluation_engine::*, reevaluation_engine::*, structs::{PositionsToEvaluate, PositionsToReevaluate}}, initial_board::*, log::FILENAME, map::Positions, piece::*, queue::*, set::Set};

fn prune_engine(run_lock: Arc<RwLock<()>>, positions: Positions, positions_to_evaluate: PositionsToEvaluate, root_board: Board) {
    let _unused = run_lock.write().unwrap();
    // println!("Pruning engine started");
    let evaluated_boards = {
        let mut evaluated_boards = positions.keys();
        let mut parent_boards = vec![root_board];
        let mut child_boards = vec![];
        let mut removed = true;
        while parent_boards.len() > 0 && removed {
            // println!("Evaluated boards: {}", evaluated_boards.len());
            removed = false;
            for parent_board in parent_boards.iter() {
                let was_present = evaluated_boards.remove(parent_board);
                if was_present {
                    removed = true;
                }
                if let Some(board_state) = positions.get(parent_board) {
                    let board_state = board_state.read().unwrap();
                    child_boards.extend(board_state.next_moves.iter().collect::<Vec<&Board>>());
                }
            }
            parent_boards = child_boards;
            child_boards = vec![];
        }
        evaluated_boards
    };
    positions.remove_keys(evaluated_boards.iter().cloned().collect::<Vec<Board>>());
    // println!("Removed unreachable boards from positions");
    let removed_from_queue = 0;
    // positions_to_evaluate.prune(&evaluated_boards);
    // println!("Removed unreachable boards from queue");
    // println!("Number of removed boards: {}", evaluated_boards.len());
    // println!("Number of removed boards from queue: {}", removed_from_queue);
    // println!("Pruning engine completed");
}

fn main() {
    unsafe {
        let f = format!("logs/{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
        let mut file_name = FILENAME.write().unwrap();
        *file_name = f;
    }

    // scratch();
    // return;

    log!("Hello, world!");
    let thread_count = std::thread::available_parallelism().unwrap().get();
    // let thread_count = 2;
    let mut app = App {
        positions: Positions::new(),
        positions_to_evaluate: DistributedQueue::new(thread_count),
        positions_to_reevaluate: Set::new(),
        run_lock:  Arc::new(RwLock::new(())),
        current_board: Arc::new(Mutex::new(INITIAL_BOARD)),
        thread_stats: Vec::with_capacity(thread_count),
        positions_evaluated_acount: Arc::new(RwLock::new(0)),
        frame_count: 0,
        input: Arc::new(RwLock::new(Input::new(String::from("")))),
        editing: true,
        prompt: String::from("Enter move:"),
    };

    for _ in 0..thread_count {
        app.thread_stats.push(ThreadStat::new());
    }

    // app.run_engine(app.thread_stats.len());
    // loop {}
    let result = app.run();
    // let result = app.run_headless();
    ratatui::restore();
    // result.unwrap();
}

#[derive(Clone)]
struct App {
    current_board: Arc<Mutex<Board>>,
    positions: Positions,
    positions_to_evaluate: PositionsToEvaluate,
    positions_to_reevaluate: PositionsToReevaluate,
    run_lock: Arc<RwLock<()>>,
    thread_stats: Vec<ThreadStat>,
    positions_evaluated_acount: Arc<RwLock<usize>>,
    frame_count: usize,
    input: Arc<RwLock<Input>>,
    editing: bool,
    prompt: String,
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
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.clone();
        let _unused = std::thread::Builder::new().name(format!("app_main")).spawn(move || {
            app.run_engine(app.thread_stats.len());
        }).unwrap();
        {
            let mut t = self.clone();
            let _used = std::thread::Builder::new().spawn(move || {
                let mut terminal = ratatui::init();
                loop {
                    terminal.draw(|frame| t.draw(frame)).unwrap();
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }).unwrap();
        }
        loop {
            
            if poll(Duration::from_millis(100))? {
                log!("Got event");
                let event = read().unwrap();

                if self.editing {
                    log!("Editing");
                    match event {
                        Event::Key(key_event) => {
                            log!("Got key event");
                            if key_event.code == KeyCode::Esc {
                                log!("Got Esc key event");
                                self.editing = false;
                            } else if key_event.code == KeyCode::Enter {
                                log!("Got Enter key event");
                                self.editing = false;
                                log!("Processing prompt");
                                self.process_prompt();
                                self.editing = true;
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
                                self.editing = true;
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
        let vertical_panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(frame.area());
        let status_panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(1)])
            .split(vertical_panes[1].inner(Margin::new(1, 1)));

        let left_panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(3),
            ])
            .split(vertical_panes[0]);

        let status_pane = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4); thread_count])
            .split(status_panes[1].inner(Margin::new(1, 1)));

        frame.render_widget(Block::default().borders(Borders::ALL), left_panes[0]);
        frame.render_widget(Block::default().borders(Borders::ALL), left_panes[1]);
        frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        
        for i in 0..self.thread_stats.len() {
            App::draw_stat(frame, i, &self.thread_stats[i], status_pane[i]);
        }

        frame.render_widget(Paragraph::new(format!("{}", self.current_board.lock().unwrap())), left_panes[0].inner(Margin::new(1, 1)));
        frame.render_widget(Block::default().borders(Borders::ALL), left_panes[1]);
        frame.render_widget(Paragraph::new(self.prompt.clone()), left_panes[1].inner(Margin::new(1, 0)));
        frame.render_widget(Paragraph::new(format!("{}", self.input.read().unwrap().value())), left_panes[1].inner(Margin::new(1, 1)));

        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[0]);

        let queue_panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1); 2])
            .split(status_panes[0]);
        let left_queue_panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); 4])
            .split(queue_panes[0]);
        let right_queue_panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); 4])
            .split(queue_panes[1]);
        frame.render_widget(Paragraph::new("Queue:"), left_queue_panes[0]);
        frame.render_widget(Paragraph::new(format!("{}", 0.separate_with_commas())).alignment(Alignment::Right), right_queue_panes[0]);
        frame.render_widget(Paragraph::new("Board Pieces:"), left_queue_panes[1]);
        let len = {
            self.positions.map.read().unwrap().len()
        };
        frame.render_widget(Paragraph::new(format!("{}", len.separate_with_commas())).alignment(Alignment::Right), right_queue_panes[1]);
        frame.render_widget(Paragraph::new("Positions evaluated:"), left_queue_panes[2]);
        frame.render_widget(Paragraph::new(format!("{}", self.positions.len().separate_with_commas())).alignment(Alignment::Right), right_queue_panes[2]);
        frame.render_widget(Paragraph::new("Positions evaluated pseudo:"), left_queue_panes[3]);
        frame.render_widget(Paragraph::new(format!("{}", self.positions_evaluated_acount.read().unwrap().separate_with_commas())).alignment(Alignment::Right), right_queue_panes[3]);
        frame.render_widget(Paragraph::new(format!("Engine status: {}", self.frame_count)), vertical_panes[1]);
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
                if let Some(board_state) = current_board_state {
                    log!("Processing prompt: Found board state for current position");

                    let board_state = board_state.read().unwrap();
    
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
            
            {
                let next_board_state = self.positions.get(&next_board);
                match next_board_state {
                    Some(next_board_state) => {
                        let next_board_state = next_board_state.read().unwrap();
                        let next_best_move = next_board_state.next_best_move.read().unwrap();
                        match *next_best_move {
                            None => {
                                log!("Processing prompt: No next best move found for entered move's position");
                                self.prompt = String::from("Have not evaluated position yet. Enter move:");
                                input.reset();
                                return;
                            }
                            Some(next_best_move) => {
                                log!("Processing prompt: Setting current board to {}", next_best_move.board);
                                log!("Setting current board to {}", next_best_move.board);
                                *current_board = next_best_move.board;
                                input.reset();
                            }
                        }
                    },
                    None => {
                        // println!("Positions: {}", positions.len());
                        // println!("Depth: {}", DEPTH.lock().unwrap());
                        log!("Processing prompt: Could not find board state for entered move's position");
                        self.prompt = String::from("Have not evaluated position yet. Enter move:");
                        input.reset();
                        return;
                    }
                }
            }
    
            // prune_engine(self.run_lock.clone(), self.positions.clone(), self.positions_to_evaluate.clone(), *current_board);
    }

    fn run_engine(&self, thread_count: usize) {
        self.positions_to_evaluate.queue(vec![(None, INITIAL_BOARD, 0)]);
        
        let mut threads: Vec<JoinHandle<()>> = Vec::new();
        // println!("Starting {} threads", cpu_count);
        for i  in 0..self.thread_stats.len() {
            let app = self.clone();
            let run_lock = self.run_lock.clone();
            let join_handle = std::thread::Builder::new().name(format!("evaluation_engine_{}", i)).spawn(move || {
                evaluation_engine(i, run_lock, app);
            }).unwrap();
            threads.push(join_handle);
        }
        {
            let positions = self.positions.clone();
            let positions_to_reevaluate = self.positions_to_reevaluate.clone();
            let run_lock = self.run_lock.clone();
            let _unused = std::thread::Builder::new().name(format!("reevaluation_engine")).spawn(move || {
                reevaluation_engine(run_lock, positions_to_reevaluate, positions);
            }).unwrap();
        }

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
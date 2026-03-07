use std::{sync::{Arc, Condvar, Mutex, RwLock, mpsc::{self, Receiver, Sender}}, thread::{JoinHandle, sleep}, time::{Duration, Instant}};

use ratatui::{Frame, crossterm::event::{Event, KeyCode, poll, read}, layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, widgets::{Block, Borders, Paragraph}};
use regex::Regex;
use thousands::Separable;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{core::{chess::{board::Board, initial_board::INITIAL_BOARD, piece::{BLACK, EMPTY, PRESENT, get_color, get_presence}}, engine::{evaluation_engine::evaluation_engine, prune_engine::prune_engine, reevaluation_engine::reevaluation_engine, structs::{PositionToEvaluate, PositionsToEvaluate, PositionsToReevaluate}}, structs::{lock::LockWaiter, map::{GroupedPositions, Positions}, queue::DistributedQueue, weighted_queue::DistributedWeightedQueue}}, log};

use serde_json;

#[derive(Clone)]
pub struct App {
    pub current_board: Arc<RwLock<Board>>,
    pub positions: GroupedPositions,
    pub positions_to_evaluate: PositionsToEvaluate,
    pub positions_to_reevaluate: PositionsToReevaluate,
    pub run_lock: Arc<RwLock<()>>,
    pub thread_stats: Vec<ThreadStat>,
    pub thread_count: usize,
    pub positions_evaluated_acount: Arc<RwLock<usize>>,
    pub frame_count: usize,
    pub input: Arc<RwLock<Input>>,
    pub editing: Arc<RwLock<bool>>,
    pub prompt: Arc<RwLock<String>>,
    pub start_time: std::time::Instant,
    pub status: Arc<RwLock<String>>,
    pub current_depth: Arc<RwLock<usize>>,
    pub waiter: LockWaiter,
}

impl App {

    pub fn new(thread_count: usize) -> App {
        let depth = Arc::new(RwLock::new(5));
        let waiter = LockWaiter::new();
        let mut app = App {
            positions: GroupedPositions::new(thread_count),
            positions_to_evaluate: DistributedWeightedQueue::new(thread_count, depth.clone(), waiter.clone()),
            positions_to_reevaluate: DistributedQueue::new(thread_count),
            run_lock:  Arc::new(RwLock::new(())),
            current_board: Arc::new(RwLock::new(INITIAL_BOARD)),
            thread_stats: Vec::with_capacity(thread_count),
            thread_count: thread_count,
            positions_evaluated_acount: Arc::new(RwLock::new(0)),
            frame_count: 0,
            input: Arc::new(RwLock::new(Input::new(String::from("")))),
            editing: Arc::new(RwLock::new(true)),
            prompt: Arc::new(RwLock::new(String::from("Enter move:"))),
            start_time: std::time::Instant::now(),
            status: Arc::new(RwLock::new(String::from("Evaluating..."))),
            current_depth: depth,
            waiter: waiter,
        };
    
        for _ in 0..thread_count {
            app.thread_stats.push(ThreadStat::new());
        }
        return app;
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.clone();
        let _unused = std::thread::Builder::new().name(format!("app_main")).spawn(move || {
            app.run_engine(app.thread_stats.len());
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
        let (prune_sender, prune_receiver) = mpsc::channel::<Board>();
        let (loop_prune_sender, loop_prune_receiver) = mpsc::channel::<()>();
        {
            let app = self.clone();
            std::thread::spawn(move || {
                prune_engine(app.clone(), prune_receiver, loop_prune_sender);
            });
        }

        let (reval_sender, reval_receiver) = mpsc::channel::<()>();
        let (loop_reval_sender, loop_reval_receiver) = mpsc::channel::<()>();
        {
            let app = self.clone();
            std::thread::spawn(move || {
                reevaluation_engine(app.clone(), reval_receiver, loop_reval_sender);
            });
        }
        log!("Starting loop");
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
                                self.process_prompt(&prune_sender, &loop_prune_receiver, &reval_sender, &loop_reval_receiver);
                                let mut editing = self.editing.write().unwrap();
                                *editing = true;
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

        frame.render_widget(Paragraph::new(format!("{}", self.current_board.read().unwrap())), board_pane.inner(Margin::new(1, 1)));
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(Paragraph::new(self.prompt.read().unwrap().clone()), prompt_pane.inner(Margin::new(1, 0)));
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
            .constraints(vec![Constraint::Length(21), Constraint::Fill(1)])
            .split(positions_evaluated_pane).as_ref().try_into().unwrap();
        let [positions_evaluated_pseudo_name_pane, positions_evaluated_pseudo_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50); 2])
            .split(positions_evaluated_pseudo_pane).as_ref().try_into().unwrap();

        frame.render_widget(Paragraph::new("Reval Queue:"), reval_queue_stat_name_pane);
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

    fn process_prompt(&mut self, prune_sender: &Sender<Board>, loop_prune_receiver: &Receiver<()>, reval_sender: &Sender<()>, loop_reval_receiver: &Receiver<()>) {
            let mut current_board = {
                *self.current_board.read().unwrap()
            };
            let re = Regex::new(r"([a-z])(\d)-([a-z])(\d)").unwrap();
            let mut input = self.input.write().unwrap();
            let captures = match re.captures(input.value()){
                Some(captures) => captures,
                None => {
                    *self.prompt.write().unwrap() = String::from("Invalid syntax. Enter move:");
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
                *self.prompt.write().unwrap() = String::from("Invalid move. Enter move:");
                input.reset();
                return;
            }

            log!("Processing prompt: Valid pieces present in source and target squares");
            log!("Processing prompt: current_board: {:?} \n{}", current_board.pieces, current_board);
            let next_board = {
                let current_board_state = self.positions.get(&current_board);
                if let Some(pointer_to_board) = current_board_state {
                    log!("Processing prompt: Found board state for current position");

                    let board_arrangement_positions = pointer_to_board.ptr.upgrade().unwrap();
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
    
                    let mut next_board: Option<Board> = None;
                    for next_move in board_state.next_moves.iter() {
                        let source_piece = next_move.0.get(7-from_rank, 7-from_file);
                        let target_piece = next_move.0.get(7-to_rank, 7-to_file);
                        // log!("Processing prompt: candidate:\n{}", next_move.inverted());
                        if get_presence(source_piece) == EMPTY && get_presence(target_piece) == PRESENT && get_color(target_piece) == BLACK {
                            next_board = Some(next_move.0);
                            break;
                        }
                    }
                    match next_board {
                        Some(next_board) => next_board,
                        None => {
                            log!("Processing prompt: Could not find move corresponding to prompt");
                            *self.prompt.write().unwrap() = String::from("Invalid move 2. Enter move:");
                            input.reset();
                            return;
                        }
                    }
                } else {
                    log!("Processing prompt: Could not find board state for current position");
                    *self.prompt.write().unwrap() = String::from("Invalid move 3. Enter move:");
                    input.reset();
                    return;
                }
            };
            drop(input);
            log!("Position: {}", serde_json::to_string(&next_board).unwrap());
            log!("Position Board Arrangement: {}", serde_json::to_string(&next_board.get_board_arrangement()).unwrap());
            let editing = self.editing.clone();
            let app = self.clone();
            let run_lock_lock = app.run_lock.write().unwrap();
            log!("Run lock locked");
            let app = self.clone();
            let start_time = Instant::now();
            log!("Player played move: {}", next_board);
            log!("Plauyer played move json: {}", serde_json::to_string(&next_board).unwrap());
            {
                if(current_board.get_board_arrangement() != next_board.get_board_arrangement()) {
                    prune_sender.send(next_board).unwrap();
                    loop_prune_receiver.recv().unwrap();
                }
            }
            {
                reval_sender.send(()).unwrap();
                loop_reval_receiver.recv().unwrap();
            }
            log!("Pruned and re-evaluated in {}s", start_time.elapsed().as_secs());
            {
                let app = app.clone();
                *app.status.write().unwrap() = String::from("Evaluating...");
            }
            {
                let mut input = self.input.write().unwrap();
                let next_board_state = self.positions.get(&next_board);
                match next_board_state {
                    Some(pointer_to_board) => {

                        let board_arrangement_positions = pointer_to_board.ptr.upgrade().unwrap();
                        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                        let next_board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();
                        let next_best_move = next_board_state.next_best_move.read().unwrap();
                        match *next_best_move {
                            None => {
                                log!("Processing prompt: No next best move found for entered move's position");
                                *self.prompt.write().unwrap() = String::from("Cannot find next best move. Enter move:");
                                input.reset();
                                return;
                            }
                            Some(next_best_move) => {
                                log!("Processing prompt: Setting current board to {}", next_best_move.board);
                                log!("Setting current board to {}", next_best_move.board);
                                *self.current_board.write().unwrap() = next_best_move.board;
                                input.reset();
                            }
                        }
                    },
                    None => {
                        // println!("Positions: {}", positions.len());
                        // println!("Depth: {}", DEPTH.lock().unwrap());
                        log!("Processing prompt: Could not find board state for entered move's position");
                        *self.prompt.write().unwrap() = String::from("Have not evaluated position yet. Enter move:");
                        input.reset();
                        return;
                    }
                }
            }
            let depth = {
                let app = self.clone();
                *(app.current_depth.read().unwrap())
            };
            {
                let app = self.clone();
                *(app.current_depth.write().unwrap()) = depth + 2;
                self.waiter.notify();
            }
            drop(run_lock_lock);
    }

    fn run_engine(&self, thread_count: usize) {
        log!("Running engine");
        self.positions_to_evaluate.queue(0, vec![PositionToEvaluate{ value: (None, INITIAL_BOARD) }]);
        log!("queued");
        let mut threads: Vec<JoinHandle<()>> = Vec::new();
        log!("Starting {} threads", thread_count);
        let (eval_sender, eval_receiver) = mpsc::channel::<(usize, Vec<PositionToEvaluate>)>();
        let q = self.positions_to_evaluate.clone();
        std::thread::Builder::new().name(String::from("eval_queuer")).spawn(move || {
            loop {
                let value = eval_receiver.recv().unwrap();
                q.queue(value.0, value.1);
            }
        }).unwrap();
        for i  in 0..self.thread_stats.len() {
            let app = self.clone();
            let run_lock = self.run_lock.clone();
            let eval_sender = eval_sender.clone();
            let join_handle = std::thread::Builder::new().name(format!("evaluation_engine_{}", i)).spawn(move || {
                evaluation_engine(i, run_lock, app, eval_sender.clone());
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
        // loop {}
    }
}

#[derive(Clone)]
pub struct ThreadStat{
    pub positions_evaluated_length: Arc<RwLock<usize>>,
    pub running_status: Arc<RwLock<bool>>,
}

impl ThreadStat {
    pub fn new() -> Self {
        ThreadStat {
            positions_evaluated_length: Arc::new(RwLock::new(0)),
            running_status: Arc::new(RwLock::new(false)),
        }
    }
}
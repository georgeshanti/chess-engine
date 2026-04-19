use std::{io::Write, ops::Deref, os::unix::thread, sync::{Arc, Condvar, Mutex, RwLock, mpsc::{self, Receiver, Sender}}, thread::{JoinHandle, sleep}, time::{Duration, Instant}};

use array_builder::ArrayBuilder;
use chrono::format::Fixed;
use crossterm::{Command, event::{Event, KeyCode, poll, read}};
use regex::Regex;
use thousands::Separable;
use tui_input::{backend::crossterm::EventHandler};

use crate::{core::{chess::{board::Board, initial_board::INITIAL_BOARD, piece::{BLACK, EMPTY, get_color, get_presence}}, draw::{Alignment, Block, Borders, Constraint, Direction, FixedLengthString, Frame, Input, Layout, Margin, RawU16Buffer, Rect, convert_usize_to_u8_string}, engine::{evaluation_engine::evaluation_engine, prune_engine::prune_engine, reevaluation_engine::reevaluation_engine, structs::{PositionToEvaluate, PositionsToEvaluate, PositionsToReevaluate}}, structs::{lock::LockWaiter, map::{GroupedPositions, Positions}, queue::DistributedQueue, weighted_queue::DistributedWeightedQueue}}, log};

use serde_json;

#[derive(Clone)]
pub struct App {
    pub computer_count: usize,
    pub queuer_count: usize,
    pub current_board: Arc<RwLock<Board>>,
    pub positions: GroupedPositions,
    pub positions_to_evaluate: PositionsToEvaluate,
    pub positions_to_reevaluate: PositionsToReevaluate,
    pub run_lock: Arc<RwLock<()>>,
    pub thread_stats: Vec<ThreadStat>,
    pub positions_evaluated_acount: Arc<RwLock<usize>>,
    pub frame_count: usize,
    pub input: Arc<RwLock<Input>>,
    pub editing: Arc<RwLock<bool>>,
    pub prompt: Arc<RwLock<FixedLengthString<64>>>,
    pub start_time: std::time::Instant,
    pub status: Arc<RwLock<FixedLengthString<64>>>,
    pub current_depth: Arc<RwLock<usize>>,
    pub waiter: LockWaiter,
}

impl App {

    pub fn new(computer_count: usize, queuer_count: usize) -> App {
        let depth = Arc::new(RwLock::new(5));
        let waiter = LockWaiter::new();
        let mut prompt = FixedLengthString::new (&[b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
        let mut app = App {
            positions: GroupedPositions::new(computer_count),
            positions_to_evaluate: DistributedWeightedQueue::new(computer_count, depth.clone(), waiter.clone()),
            positions_to_reevaluate: DistributedQueue::new(computer_count),
            run_lock:  Arc::new(RwLock::new(())),
            current_board: Arc::new(RwLock::new(INITIAL_BOARD)),
            thread_stats: Vec::with_capacity(computer_count),
            computer_count: computer_count,
            queuer_count: queuer_count,
            positions_evaluated_acount: Arc::new(RwLock::new(0)),
            frame_count: 0,
            input: Arc::new(RwLock::new(Input::new())),
            editing: Arc::new(RwLock::new(true)),
            prompt: Arc::new(RwLock::new(prompt)),
            start_time: std::time::Instant::now(),
            status: Arc::new(RwLock::new(FixedLengthString::new(b"Evaluating..."))),
            current_depth: depth,
            waiter: waiter,
        };
    
        for _ in 0..computer_count {
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
                let mut frame = Frame::new();
                loop {
                    t.draw(&mut frame);
                    frame.stdout.flush();
                    std::thread::sleep(Duration::from_millis(1000));
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

    pub fn draw(&mut self, frame: &mut Frame) {
        let _unused = self.run_lock.write().unwrap();
        let thread_count = self.thread_stats.len();
        let [left_pane, right_pane] = Layout::default()
        // let l = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(frame.area()).deref().try_into().unwrap();
        let [global_status_pane, thread_status_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(7), Constraint::Fill(1)])
            .split(right_pane.inner(Margin{m: 1})).deref().try_into().unwrap();
        let [board_pane, prompt_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[
                Constraint::Fill(1),
                Constraint::Length(3),
            ])
            .split(left_pane).deref().try_into().unwrap();
        let status_pane = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(4); 124][0..thread_count])
            .split(thread_status_pane.inner(Margin{m:1}));

        frame.render_widget(Block::default().borders(Borders::ALL), board_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), right_pane);
        
        for i in 0..self.thread_stats.len() {
            App::draw_stat(frame, i, &self.thread_stats[i], status_pane[i]);
        }
        frame.render_widget(RawU16Buffer{buf: self.current_board.read().unwrap().d()}, board_pane.inner(Margin{m:1}));
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(*self.prompt.read().unwrap(), prompt_pane);
        frame.render_widget(FixedLengthString::<64>::new(self.input.read().unwrap().value.deref()), prompt_pane.inner(Margin{m:1}));

        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[0]);

        let [eval_queue_stat_pane, reval_queue_stat_pane, board_pieces_pane, positions_evaluated_pane, positions_evaluated_pseudo_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(4), Constraint::Length(1)])
            .split(global_status_pane).deref().try_into().unwrap();

        let [eval_queue_stat_name_pane, eval_queue_stat_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Fill(1), Constraint::Fill(1)])
            .split(eval_queue_stat_pane).deref().try_into().unwrap();
        frame.render_widget(FixedLengthString::<11>::new(&[b'E', b'v', b'a', b'l', b' ', b'Q', b'u', b'e', b'u', b'e', b':']), eval_queue_stat_name_pane);

        let [reval_queue_stat_name_pane, reval_queue_stat_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Fill(1), Constraint::Fill(1)])
            .split(reval_queue_stat_pane).deref().try_into().unwrap();
        let [board_pieces_name_pane, board_pieces_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Percentage(50); 2])
            .split(board_pieces_pane).deref().try_into().unwrap();
        let [positions_evaluated_name_pane, positions_evaluated_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Length(22), Constraint::Fill(1)])
            .split(positions_evaluated_pane).deref().try_into().unwrap();
        let [positions_evaluated_pseudo_name_pane, positions_evaluated_pseudo_value_pane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Percentage(50); 2])
            .split(positions_evaluated_pseudo_pane).deref().try_into().unwrap();

        frame.render_widget(FixedLengthString::<12>::new(&[b'R', b'e', b'v', b'a', b'l', b' ', b'Q', b'u', b'e', b'u', b'e', b':']), reval_queue_stat_name_pane);
        let mut length = 0;
        for i in 0..self.thread_stats.len() {
            let l = self.positions_to_reevaluate.queues[i].length.read().unwrap();
            length += *l;
        }
        frame.render_widget(convert_usize_to_u8_string(length).alignment(Alignment::Right), reval_queue_stat_value_pane);

        let lengths = self.positions_to_evaluate.lengths();
        let mut lengths_string = FixedLengthString::<86>::new(&[0; 0]);
        lengths_string.add(FixedLengthString::<1>::new(&[b'{']));
        let mut i = 0;
        for length in lengths.iter() {
            if i > 1 {
                break
            }
            let index_buf = convert_usize_to_u8_string(*length.0);
            lengths_string.add(index_buf);
            lengths_string.add(FixedLengthString::<1>::new(&[b':']));
            lengths_string.add(FixedLengthString::<1>::new(&[b' ']));

            let length_buf = convert_usize_to_u8_string(*length.1);
            lengths_string.add(length_buf);
            lengths_string.add(FixedLengthString::<1>::new(&[b',']));
            lengths_string.add(FixedLengthString::<1>::new(&[b' ']));
            i += 1;
        }
        lengths_string.add_u8(&[b'}']);
        frame.render_widget(lengths_string.alignment(Alignment::Right), eval_queue_stat_value_pane);
        frame.render_widget(FixedLengthString::<20>::new(&[b'P', b'o', b's', b'i', b't', b'i', b'o', b'n', b's', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd', b':']), positions_evaluated_name_pane);
        let positions_len = {
            let mut positions_len = FixedLengthString::<512>::new(&[0; 0]);
            positions_len.add_u8(&[b'{']);
            let lens = self.positions.len();
            for i in 0..self.positions.length {
                let index_buf = convert_usize_to_u8_string(i);
                positions_len.add(index_buf);
                positions_len.add_u8(&[b':']);
                positions_len.add_u8(&[b' ']);

                let length_buf = convert_usize_to_u8_string(lens[i].1);
                positions_len.add(length_buf);
                positions_len.add_u8(&[b',']);
                positions_len.add_u8(&[b' ']);
            }
            positions_len.add_u8(&[b'}']);
            positions_len
        };
        frame.render_widget(positions_len.alignment(Alignment::Right), positions_evaluated_value_pane);
        frame.render_widget(FixedLengthString::<27>::new(&[b'P', b'o', b's', b'i', b't', b'i', b'o', b'n', b's', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd', b' ', b'p', b's', b'e', b'u', b'd', b'o', b':']), positions_evaluated_pseudo_name_pane);
        frame.render_widget(convert_usize_to_u8_string(*self.positions_evaluated_acount.read().unwrap()).alignment(Alignment::Right), positions_evaluated_pseudo_value_pane);
        let status = {
            let mut buf = FixedLengthString::<64>::new(&[b' '; 0]);
            buf.add_u8(&[b'T', b'i', b'm', b'e', b':', b' ']);
            buf.add(convert_usize_to_u8_string(self.start_time.elapsed().as_secs() as usize));
            buf.add_u8(&[b' ', b'E', b'n', b'g', b'i', b'n', b'e', b' ', b's', b't', b'a', b't', b'u', b's', b':', b' ']);
            let status = { *self.status.read().unwrap() };
            buf.add(status);
            buf
        };
        frame.render_widget(status, right_pane);
        self.frame_count = self.frame_count + 1;
    }

    fn draw_stat(frame: &mut Frame, index: usize,thread_stat: &ThreadStat, rect: Rect) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Percentage(50); 2])
            .split(rect.inner(Margin{m:1}));
        let left_bars = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(1); 2])
            .split(panes[0]);
        let right_bars = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(1); 2])
            .split(panes[1]);
        frame.render_widget(Block::default().borders(Borders::ALL), rect);
        let thread_number = {
            let mut f = FixedLengthString::<46>::new(&[b' '; 24]);
            f.buf[0..8].copy_from_slice(&[b'T', b'h', b'r', b'e', b'a', b'd', b' ', b'#']);
            let number = convert_usize_to_u8_string(index);
            f.buf[8..46].copy_from_slice(&number.buf);
            f
        };
        frame.render_widget(thread_number, rect.inner(Margin{m:1}));
        frame.render_widget(FixedLengthString::<5>::new(&match * thread_stat.running_status.read().unwrap() { false => [b'f', b'a', b'l', b's', b'e'], true => [b't', b'r', b'u', b'e', b' '], }).alignment(Alignment::Right), right_bars[0]);
        frame.render_widget(FixedLengthString::<19>::new(&[b'P', b'o', b's', b'i', b't', b'i', b'o', b'n', b's', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd']), left_bars[1]);
        frame.render_widget(convert_usize_to_u8_string(*thread_stat.positions_evaluated_length.read().unwrap()).alignment(Alignment::Right), right_bars[1]);
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
                    *self.prompt.write().unwrap() = FixedLengthString::new(&[b'I', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b's', b'y', b'n', b't', b'a', b'x', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
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
                *self.prompt.write().unwrap() = FixedLengthString::new(&[b'I', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b'm', b'o', b'v', b'e', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
                input.reset();
                return;
            }

            log!("Processing prompt: Valid pieces present in source and target squares");
            log!("Processing prompt: current_board: {:?} \n{:?}", current_board.pieces, current_board.d());
            let next_board = {
                let current_board_state = self.positions.get(&current_board);
                if let Some(pointer_to_board) = current_board_state {
                    log!("Processing prompt: Found board state for current position");

                    let board_arrangement_positions = pointer_to_board.ptr.upgrade().unwrap();
                    let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
                    let board_state = readable_board_arrangement_positions.get(pointer_to_board.index).read().unwrap();

                    let mut next_board: Option<Board> = None;
                    log!("board_state.next_moves: {:?}", board_state.next_moves);
                    let next_moves = readable_board_arrangement_positions.get_next_moves(board_state.next_moves.0, board_state.next_moves.1, true);
                    log!("next_moves: {}", next_moves.len());
                    for next_moves in next_moves {
                        log!("next_moves_len: {}", next_moves.len());
                        let mut found = false;
                        for next_move in next_moves {
                            let source_piece = next_move.0.get(7-from_rank, 7-from_file);
                            let target_piece = next_move.0.get(7-to_rank, 7-to_file);
                            // log!("Processing prompt: candidate:\n{}", next_move.inverted());
                            if get_presence(source_piece) == EMPTY && get_presence(target_piece) != EMPTY && get_color(target_piece) == BLACK {
                                next_board = Some(next_move.0);
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    match next_board {
                        Some(next_board) => next_board,
                        None => {
                            log!("Processing prompt: Could not find move corresponding to prompt");
                            *self.prompt.write().unwrap() = FixedLengthString::new(&[b'I', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b'm', b'o', b'v', b'e', b' ', b'2', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
                            input.reset();
                            return;
                        }
                    }
                } else {
                    log!("Processing prompt: Could not find board state for current position");
                    *self.prompt.write().unwrap() = FixedLengthString::new(&[b'I', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b'm', b'o', b'v', b'e', b' ', b'3', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
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
            log!("Player played move: {:?}", next_board.d());
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
                *app.status.write().unwrap() = FixedLengthString::new(b"Evaluating...");
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
                                *self.prompt.write().unwrap() = FixedLengthString::new(&[b'C', b'a', b'n', b'n', b'o', b't', b' ', b'f', b'i', b'n', b'd', b' ', b'n', b'e', b'x', b't', b' ', b'b', b'e', b's', b't', b' ', b'm', b'o', b'v', b'e', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
                                input.reset();
                                return;
                            }
                            Some(next_best_move) => {
                                log!("Processing prompt: Setting current board to {:?}", next_best_move.board.d());
                                log!("Setting current board to {:?}", next_best_move.board.d());
                                *self.current_board.write().unwrap() = next_best_move.board;
                                input.reset();
                            }
                        }
                    },
                    None => {
                        // println!("Positions: {}", positions.len());
                        // println!("Depth: {}", DEPTH.lock().unwrap());
                        log!("Processing prompt: Could not find board state for entered move's position");
                        *self.prompt.write().unwrap() = FixedLengthString::new(&[b'H', b'a', b'v', b'e', b' ', b'n', b'o', b't', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd', b' ', b'p', b'o', b's', b'i', b't', b'i', b'o', b'n', b' ', b'y', b'e', b't', b'.', b' ', b'E', b'n', b't', b'e', b'r', b' ', b'm', b'o', b'v', b'e', b':']);
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
        let mut ba: ArrayBuilder<PositionToEvaluate, 1> = ArrayBuilder::new();
        ba.push(PositionToEvaluate{ value: (None, INITIAL_BOARD) });
        self.positions_to_evaluate.queue(0, ba.iter().as_slice());
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
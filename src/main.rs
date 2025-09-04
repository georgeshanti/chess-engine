mod core;

use std::{cmp::Ordering, collections::{HashMap, HashSet}, io, sync::{Arc, Mutex, RwLock}, thread::JoinHandle, time::Duration};
use ratatui::{layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, text::Text, widgets::{Block, Borders, Paragraph}, DefaultTerminal, Frame};
use regex::Regex;
use crossterm::event::{self, Event, KeyCode};

use crate::core::{board::*, board_state::*, initial_board::*, piece::*, queue::*};

type Positions = Arc<RwLock<HashMap<Board, Arc<RwLock<BoardState>>>>>;
type PositionsToEvaluate = Queue<(Option<Board>, Board, usize)>;
type PositionsToReevaluate = Queue<Board>;

const RUN_DURATION: Duration = Duration::from_secs(1);

static DEPTH: Mutex<usize> = Mutex::new(0);

fn evaluation_engine(index: usize, run_lock: Arc<RwLock<()>>, app: App) {
    // while time elapsed is less than 10 seconds
    // println!("Evaluation engine started");
    let start_time = std::time::Instant::now();
    // while start_time.elapsed() < RUN_DURATION {
    let positions_to_evaluate = app.positions_to_evaluate.clone();
    let positions = app.positions.clone();
    let positions_to_reevaluate = app.positions_to_reevaluate.clone();
    loop {
        {
            *(app.thread_stats[index].running_status.write().unwrap()) = false;
        }

        std::thread::sleep(Duration::from_millis(10000));
        let _unused = run_lock.read().unwrap();

        {
            *(app.thread_stats[index].running_status.write().unwrap()) = true;
        }
        // println!("Evaluation engine running");
        let (previous_board, board, board_depth) = positions_to_evaluate.dequeue();
        {
            let mut positions_evaluated_length = app.thread_stats[index].positions_evaluated_length.write().unwrap();
            *positions_evaluated_length = *positions_evaluated_length + 1;
        }
        {
            let mut current_highest_depth = DEPTH.lock().unwrap();
            if *current_highest_depth < board_depth {
                *current_highest_depth = board_depth;
            }
        }
        // println!("Evaluation engine dequeued: {}", engine_id);
        let readable_positions = positions.read().unwrap();
        if let Some(parent) = previous_board {
            if readable_positions.get(&parent).is_none() {
                // println!("Parent not found: {}", parent);
                continue;
            }
        }
        let board_state = readable_positions.get(&board);
        match board_state {
            None => {
                // println!("Evaluation engine inserting");
                drop(readable_positions);
                let mut writable_positions = positions.write().unwrap();
                let new_board_state = Arc::new(RwLock::new(BoardState::new()));
                writable_positions.insert(board, new_board_state.clone());
                drop(writable_positions);
                let evaluated_board_state = board.get_evaluation();

                let mut writable_board_state = new_board_state.write().unwrap();
                writable_board_state.self_evaluation = evaluated_board_state.0;
                writable_board_state.next_moves = evaluated_board_state.1.clone();
                match previous_board {
                    Some(previous_board) => {
                        writable_board_state.previous_moves.write().unwrap().insert(previous_board);
                        positions_to_reevaluate.queue(vec![previous_board]);
                    },
                    _ => {}
                };
                drop(writable_board_state);

                let mut next_boards: Vec<(Option<Board>, Board, usize)> = Vec::new();
                for next_board in evaluated_board_state.1 {
                    match positions.read().unwrap().get(&next_board) {
                        None => {
                            next_boards.push((Some(board), next_board, board_depth + 1));
                        },
                        Some(board_state) => {
                            append_parent(board_state, &previous_board, &positions_to_reevaluate);
                        }
                    }
                    // next_boards.push((Some(board), next_board, board_depth + 1));
                }
                positions_to_evaluate.queue(next_boards);

                // println!("Evaluation engine inserted");
            },
            Some(board_state) => {
                // println!("Evaluation engine reading");
                append_parent(&board_state, &previous_board, &positions_to_reevaluate);
            },
        }
        // println!("Evaluation engine completed: {}", engine_id);
    }
    // println!("Evaluation engine completed");
}

fn append_parent(board_state: &Arc<RwLock<BoardState>>, previous_board: &Option<Board>, positions_to_reevaluate: &Queue<Board>) {
    let writable_board_state = board_state.read().unwrap();
    match previous_board {
        Some(previous_board) => {
            let inserted = writable_board_state.previous_moves.write().unwrap().insert(*previous_board);
            if inserted {
                let mut previous_boards: Vec<Board> = Vec::new();
                for previous_board in writable_board_state.previous_moves.read().unwrap().iter() {
                    previous_boards.push(*previous_board);
                }
                positions_to_reevaluate.queue(previous_boards);
            }
        },
        _ => {}
    };
}

fn reevaluation_engine(run_lock: Arc<RwLock<()>>, positions_to_reevaluate: Queue<Board>, positions: Positions) {
    // println!("Re-Evaluation engine started");
    let start_time = std::time::Instant::now();
    // while start_time.elapsed() < RUN_DURATION {
    loop {
        let _unused = run_lock.read().unwrap();
        // println!("Reeval running");
        let board_to_reevaluate = positions_to_reevaluate.dequeue();
        let positions = positions.read().unwrap();

        if let Some(board_state) = positions.get(&board_to_reevaluate) {
            let board_state = board_state.read().unwrap();
            let mut next_best_move = board_state.next_best_move.write().unwrap();

            let mut new_next_best_move: Option<NextBestMove> = None;

            for next_position in board_state.next_moves.iter() {
                if let Some(next_position_board_state) = positions.get(&next_position) {
                    let next_position_board_state = next_position_board_state.read().unwrap();
                    let next_position_best_evaluation =  next_position_board_state.next_best_move.read().unwrap();
                    let next_position_best_evaluation = match *next_position_best_evaluation {
                        Some(next_position_best_evaluation) => next_position_best_evaluation.evaluation,
                        None => next_position_board_state.self_evaluation,
                    }.invert();
                    match new_next_best_move {
                        None => {
                            new_next_best_move = Some(NextBestMove{
                                board: *next_position,
                                evaluation: next_position_best_evaluation,
                            });
                        }
                        Some(current_next_best_move) => {
                            if current_next_best_move.evaluation.compare_to(&next_position_best_evaluation) == Ordering::Less {
                                new_next_best_move = Some(NextBestMove{
                                    board: *next_position,
                                    evaluation: next_position_best_evaluation,
                                });
                            }
                        }
                    }
                }
            }
            if let Some(new_next_best_move) = new_next_best_move {
                if Some(new_next_best_move) != *next_best_move {
                    // println!("Updating best move");
                    *next_best_move = Some(new_next_best_move);

                    let mut previous_boards: Vec<Board> = Vec::new();
                    for previous_board in board_state.previous_moves.read().unwrap().iter() {
                        previous_boards.push(*previous_board);
                    }
                    positions_to_reevaluate.queue(previous_boards);
                } else {
                    // println!("Not updating best move #1");
                }
            }
        }
    }
}

fn prune_engine(run_lock: Arc<RwLock<()>>, positions: Positions, positions_to_evaluate: PositionsToEvaluate, root_board: Board) {
    let _unused = run_lock.write().unwrap();
    // println!("Pruning engine started");
    let evaluated_boards = {
        let positions = positions.read().unwrap();
        let mut evaluated_boards = positions.keys().cloned().collect::<HashSet<Board>>();
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
    // println!("Found reachable boards");
    {
        let mut positions = positions.write().unwrap();
        for board in evaluated_boards.iter() {
            positions.remove(board);
        }
    }
    // println!("Removed unreachable boards from positions");
    let removed_from_queue = 0;
    // positions_to_evaluate.prune(&evaluated_boards);
    // println!("Removed unreachable boards from queue");
    // println!("Number of removed boards: {}", evaluated_boards.len());
    // println!("Number of removed boards from queue: {}", removed_from_queue);
    // println!("Pruning engine completed");
}

fn main() {
    // println!("Hello, world!");
    let thread_count = std::thread::available_parallelism().unwrap().get();
    let app = App {
        positions: Arc::new(RwLock::new(HashMap::new())),
        positions_to_evaluate: Queue::new(),
        positions_to_reevaluate: Queue::new(),
        run_lock:  Arc::new(RwLock::new(())),
        current_board: Arc::new(Mutex::new(INITIAL_BOARD)),
        thread_stats: vec![ThreadStat::new(); thread_count],
    };

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result.unwrap();
}

#[derive(Clone)]
struct App {
    current_board: Arc<Mutex<Board>>,
    positions: Positions,
    positions_to_evaluate: PositionsToEvaluate,
    positions_to_reevaluate: PositionsToReevaluate,
    run_lock: Arc<RwLock<()>>,
    thread_stats: Vec<ThreadStat>,
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
    fn run(&self, terminal: &mut DefaultTerminal) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.clone();
        let _unused = std::thread::Builder::new().name(format!("app_main")).spawn(move || {
            app.run_engine(app.thread_stats.len());
        }).unwrap();
        
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if matches!(event::read()?, Event::Key(_)) {
                break Ok(());
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        // println!("Drawing");
        let thread_count = self.thread_stats.len();
        let vertical_panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(frame.area());

        let left_panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(vertical_panes[0]);

        let status_pane = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4); thread_count])
            .split(vertical_panes[1].inner(Margin::new(1, 1)));

        frame.render_widget(Block::default().borders(Borders::ALL), left_panes[0]);
        frame.render_widget(Block::default().borders(Borders::ALL), left_panes[1]);
        frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        
        for i in 0..self.thread_stats.len() {
            App::draw_stat(frame, i, &self.thread_stats[i], status_pane[i]);
        }

        frame.render_widget(Paragraph::new(format!("{}", self.current_board.lock().unwrap())), left_panes[1].inner(Margin::new(1, 1)));
        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[0]);
        frame.render_widget(Paragraph::new("Engine status"), vertical_panes[1]);
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
        frame.render_widget(Paragraph::new("Status"), left_bars[0]);
        frame.render_widget(Paragraph::new(format!("{}", thread_stat.running_status.read().unwrap())).alignment(Alignment::Right), right_bars[0]);
        frame.render_widget(Paragraph::new("Positions evaluated"), left_bars[1]);
        frame.render_widget(Paragraph::new(format!("{}", thread_stat.positions_evaluated_length.read().unwrap())).alignment(Alignment::Right), right_bars[1]);
    }

    fn run_engine(&self, thread_count: usize) {
        self.positions_to_evaluate.queue(vec![(None, INITIAL_BOARD, 0)]);
    
        let cpu_count;
        cpu_count = std::thread::available_parallelism().unwrap().get()-2;
        
        let mut threads: Vec<JoinHandle<()>> = Vec::new();
        // println!("Starting {} threads", cpu_count);
        for i  in 0..thread_count {
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
    
        loop {
            let mut current_board = self.current_board.lock().unwrap();
            // println!("{}", current_board);
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            let re = Regex::new(r"(\d)-(\d)-(\d)-(\d)").unwrap();
            // extract the numbers from the buffer
            let captures = re.captures(&buffer).unwrap();
            let from_file = captures[1].parse::<usize>().unwrap();
            let from_rank = captures[2].parse::<usize>().unwrap();
            let to_file = captures[3].parse::<usize>().unwrap();
            let to_rank = captures[4].parse::<usize>().unwrap();
            // println!("from_rank:{:?}", from_rank);
            // println!("from_file:{:?}", from_file);
            // println!("to_rank:{:?}", to_rank);
            // println!("to_file:{:?}", to_file);
            let source_piece = current_board.get(from_rank, from_file);
            let target_piece = current_board.get(to_rank, to_file);
            // println!("source:{:?}", char(source_piece));
            // println!("target:{:?}", char(target_piece));
            // println!("{} {} {} {}", get_presence(source_piece) == EMPTY, get_color(source_piece) == BLACK, get_presence(target_piece) == EMPTY, get_color(target_piece) == BLACK);
            if get_presence(source_piece) == EMPTY || get_color(source_piece) == BLACK || !(get_presence(target_piece) == EMPTY || get_color(target_piece) == BLACK) {
                // println!("Invalid move 1");
                continue;
            }
    
            let next_board = {
                let positions = self.positions.read().unwrap();
                let current_board_state = positions.get(&*current_board);
                if let Some(board_state) = current_board_state {
                    let board_state = board_state.read().unwrap();
    
                    let mut next_board: Option<Board> = None;
                    for next_move in board_state.next_moves.iter() {
                        let source_piece = next_move.get(7-from_rank, 7-from_file);
                        let target_piece = next_move.get(7-to_rank, 7-to_file);
                        if get_presence(source_piece) == EMPTY && get_presence(target_piece) == PRESENT && get_color(target_piece) == BLACK {
                            next_board = Some(*next_move);
                            break;
                        }
                    }
                    match next_board {
                        Some(next_board) => next_board,
                        None => {
                            // println!("Invalid move 2");
                            continue;
                        }
                    }
                } else {
                    // println!("Invalid move 3");
                    continue;
                }
            };
            
            {
                let positions = self.positions.write().unwrap();
                let next_board_state = positions.get(&next_board);
                match next_board_state {
                    Some(next_board_state) => {
                        let next_board_state = next_board_state.read().unwrap();
                        let next_best_move = next_board_state.next_best_move.read().unwrap();
                        match *next_best_move {
                            None => {
                                panic!("No next best move");
                            }
                            Some(next_best_move) => {
                                *current_board = next_best_move.board;
                            }
                        }
                    },
                    None => {
                        // println!("Positions: {}", positions.len());
                        // println!("Depth: {}", DEPTH.lock().unwrap());
                        panic!("Have not evaluated position yet");
                    }
                }
            }
    
            prune_engine(self.run_lock.clone(), self.positions.clone(), self.positions_to_evaluate.clone(), *current_board);
        }
    }
}
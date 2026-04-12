#![feature(sync_unsafe_cell)]

#[global_allocator]
static GLOBAL: CustomAlloc<System> = CustomAlloc{allocator: System{}};

mod core;

use std::{alloc::System, io::{Write, stdout}, ops::Deref, os::unix::thread, ptr, thread::sleep, time::Duration};


use crossterm::{ExecutableCommand, QueueableCommand, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};

use crate::core::{app::App, chess::{board::{Board, BoardArrangement}, initial_board::INITIAL_BOARD, piece::char}, draw::{Block, Borders, Constraint, Direction, Frame, Layout, Margin}, log::FILENAME, mem::alloc::{CustomAlloc, convert_to_hex, wait}, structs::queue::Queue};

// fn draw(frame: &mut Frame) {
//     frame.render_widget(, frame.area());
// }

fn main() {

    unsafe {
        // let f = format!("logs/{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
        // let mut file_name = FILENAME.write().unwrap();
        // *file_name = f;

        // match std::env::var("LOG") {
        //     Ok(value) => {
        //         if value == "false" {
        //             let mut enable_log = crate::core::log::ENABLE_LOG.write().unwrap();
        //             *enable_log = false;
        //         }
        //     },
        //     Err(_) => {},
        // };

        // match std::env::var("TIMED") {
        //     Ok(value) => {
        //         if value == "true" {
        //             let mut timed = crate::core::engine::evaluation_engine::TIMED.write().unwrap();
        //             *timed = true;
        //         }
        //     },
        //     Err(_) => {},
        // };

        // let board: Board = serde_json::from_str("{\"pieces\":[144,0,160,176,168,160,152,144,136,136,136,136,136,136,136,136,0,0,152,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,200,200,0,0,0,0,0,0,0,0,0,0,0,200,200,200,0,0,200,200,200,208,216,224,240,232,224,216,208]}").unwrap();
        // *move_board.write().unwrap() = board;
    }

    let mut stdout = stdout();
    enable_raw_mode();
    stdout.execute(EnterAlternateScreen);

    // let mut app = App::new(14, 2);
    let mut frame = Frame::new();
    draw(&mut frame);
    stdout.flush();
    // std::thread::sleep(Duration::from_millis(5000));
    stdout.execute(LeaveAlternateScreen);
    disable_raw_mode();
    return;


    // let init = [INITIAL_BOARD; 1];
    // let q: Queue<Board, 10> = Queue::new();
    // q.queue(&init);
    // let mut init: [Board; 10] = [Board::new(); 10];
    // let len = q.dequeue_optional(&mut init);
    // println!("Len: {}\nBoard:\n{}\n", len, init[0]);
    // return;


    // scratch();
    // return;

    // log!("Hello, world!");
    // let thread_count = std::thread::available_parallelism().unwrap().get();
    // // let thread_count = 6;
    // let computer_count = 6;
    // let queuer_count = 1;
    // let mut app = App::new(14, 2);

    // let _ = app.run();
    // ratatui::restore();
}

    pub fn draw(frame: &mut Frame) {
        // let _unused = self.run_lock.write().unwrap();
        // let thread_count = self.thread_stats.len();
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
            .constraints(&[Constraint::Length(5), Constraint::Fill(1)])
            .split(right_pane.inner(Margin{m: 1})).deref().try_into().unwrap();

        let [board_pane, prompt_pane] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[
                Constraint::Fill(1),
                Constraint::Length(3),
            ])
            .split(left_pane).deref().try_into().unwrap();
        // let [status_pane] = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints(&[Constraint::Length(4); 124][0..thread_count])
        //     .split(thread_status_pane.inner(Margin{m:1})).deref().try_into().unwrap();

        frame.render_widget(Block::default().borders(Borders::ALL), board_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        frame.render_widget(Block::default().borders(Borders::ALL), right_pane);
        
        // for i in 0..self.thread_stats.len() {
        //     App::draw_stat(frame, i, &self.thread_stats[i], status_pane[i]);
        // }
        // frame.render_widget(RawU16Buffer{buf: self.current_board.read().unwrap().d()}, board_pane.inner(Margin{m:1}));
        // frame.render_widget(Block::default().borders(Borders::ALL), prompt_pane);
        // frame.render_widget(*self.prompt.read().unwrap(), prompt_pane.inner(Margin{m:1}));
        // frame.render_widget(FixedLengthString::<64>::new(self.input.read().unwrap().value.deref()), prompt_pane.inner(Margin{m:1}));

        // // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[1]);
        // // frame.render_widget(Block::default().borders(Borders::ALL), vertical_panes[0]);

        // let [eval_queue_stat_pane, reval_queue_stat_pane, board_pieces_pane, positions_evaluated_pane, positions_evaluated_pseudo_pane] = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints(&[Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        //     .split(global_status_pane).deref();

        // let [eval_queue_stat_name_pane, eval_queue_stat_value_pane] = *Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(&[Constraint::Fill(1), Constraint::Fill(1)])
        //     .split(*eval_queue_stat_pane).deref();

        // let [reval_queue_stat_name_pane, reval_queue_stat_value_pane] = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(&[Constraint::Fill(1), Constraint::Fill(1)])
        //     .split(*reval_queue_stat_pane).deref();
        // let [board_pieces_name_pane, board_pieces_value_pane] = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(&[Constraint::Percentage(50); 2])
        //     .split(*board_pieces_pane).deref();
        // let [positions_evaluated_name_pane, positions_evaluated_value_pane] = *Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(&[Constraint::Length(21), Constraint::Fill(1)])
        //     .split(*positions_evaluated_pane).deref();
        // let [positions_evaluated_pseudo_name_pane, positions_evaluated_pseudo_value_pane] = *Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(&[Constraint::Percentage(50); 2])
        //     .split(*positions_evaluated_pseudo_pane).deref();

        // frame.render_widget(FixedLengthString::<12>::new(&[b'R', b'e', b'v', b'a', b'l', b' ', b'Q', b'u', b'e', b'u', b'e', b':']), reval_queue_stat_name_pane);
        // let mut length = 0;
        // for i in 0..self.thread_stats.len() {
        //     let l = self.positions_to_reevaluate.queues[i].length.read().unwrap();
        //     length += *l;
        // }
        // frame.render_widget(convert_usize_to_u8_string(length).alignment(Alignment::Right), reval_queue_stat_value_pane);

        // frame.render_widget(FixedLengthString::<11>::new(&[b'E', b'v', b'a', b'l', b' ', b'Q', b'u', b'e', b'u', b'e', b':']), eval_queue_stat_name_pane);
        // let lengths = self.positions_to_evaluate.lengths();
        // let mut lengths_string = FixedLengthString::<72>::new(&[0; 72]);
        // lengths_string.buf[0] = b'{';
        // let mut i = 0;
        // for length in lengths.iter() {
        //     if i > 1 {
        //         break
        //     }
        //     let index_buf = convert_usize_to_u8_string(*length.0);
        //     lengths_string.buf[(1+(i*35))..(1+(i*35)+16)].copy_from_slice(&index_buf.buf[0..index_buf.length]);
        //     lengths_string.buf[(1+(i*35)+16)] = b':';
        //     lengths_string.buf[(1+(i*35)+16+1)] = b' ';

        //     let length_buf = convert_usize_to_u8_string(*length.1);
        //     lengths_string.buf[(1+(i*35)+16+2)..(1+(i*35)+16+2+16)].copy_from_slice(&length_buf.buf[0..length_buf.length]);
        //     lengths_string.buf[(1+(i*35)+16+2+16)] = b',';
        //     lengths_string.buf[(1+(i*35)+16+2+16+1)] = b' ';
        //     i += 1;
        // }
        // lengths_string.buf[71] = b'}';
        // frame.render_widget(lengths_string.alignment(Alignment::Right), eval_queue_stat_value_pane);
        // frame.render_widget(FixedLengthString::<20>::new(&[b'P', b'o', b's', b'i', b't', b'i', b'o', b'n', b's', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd', b':']), positions_evaluated_name_pane);
        // let positions_len = {
        //     let mut positions_len = FixedLengthString::<512>::new(&[0; 512]);
        //     positions_len.buf[0] = b'{';
        //     let lens = self.positions.len();
        //     for i in 0..self.positions.length {
        //         let index_buf = convert_usize_to_u8_string(i);
        //         positions_len.buf[(1+(i*35))..(1+(i*35)+16)].copy_from_slice(&index_buf.buf[0..index_buf.length]);
        //         positions_len.buf[(1+(i*35)+16)] = b':';
        //         positions_len.buf[(1+(i*35)+16+1)] = b' ';

        //         let length_buf = convert_usize_to_u8_string(lens[i].1);
        //         positions_len.buf[(1+(i*35)+16+2)..(1+(i*35)+16+2+16)].copy_from_slice(&length_buf.buf[0..length_buf.length]);
        //         positions_len.buf[(1+(i*35)+16+2+16)] = b',';
        //         positions_len.buf[(1+(i*35)+16+2+16+1)] = b' ';
        //     }
        //     positions_len.buf[(1+(self.positions.length*35))] = b'}';
        //     positions_len
        // };
        // frame.render_widget(positions_len.alignment(Alignment::Right), positions_evaluated_value_pane);
        // frame.render_widget(FixedLengthString::<27>::new(&[b'P', b'o', b's', b'i', b't', b'i', b'o', b'n', b's', b' ', b'e', b'v', b'a', b'l', b'u', b'a', b't', b'e', b'd', b' ', b'p', b's', b'e', b'u', b'd', b'o', b':']), positions_evaluated_pseudo_name_pane);
        // frame.render_widget(convert_usize_to_u8_string(*self.positions_evaluated_acount.read().unwrap()).alignment(Alignment::Right), positions_evaluated_pseudo_value_pane);
        // let status = {
        //     let mut buf = FixedLengthString::<64>::new(&[b' '; 64]);
        //     buf.buf[0..6].copy_from_slice(&[b'T', b'i', b'm', b'e', b':', b' ']);
        //     let time = convert_usize_to_u8_string(self.start_time.elapsed().as_secs() as usize);
        //     buf.buf[6..6+16].copy_from_slice(&time.buf[0..time.length]);
        //     buf.buf[6+16..6+16+16].copy_from_slice(&[b' ', b'E', b'n', b'g', b'i', b'n', b'e', b' ', b's', b't', b'a', b't', b'u', b's', b':', b' ']);
        //     let status = { self.status.read().unwrap() };
        //     let status = status.as_bytes();
        //     buf.buf[6+16+16..6+16+16+status.len()].copy_from_slice(status);
        //     buf
        // };
        // frame.render_widget(status, right_pane);
        // self.frame_count = self.frame_count + 1;
    }
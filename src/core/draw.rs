use std::{io::{Stdout, Write, stdout}, rc::Rc};

use array_builder::ArrayBuilder;
use crossterm::{Command, QueueableCommand, cursor::MoveTo, event::Event, terminal::size};

use crate::{core::log, log};

pub struct Frame {
    width: u16,
    height: u16,
    stdout: Stdout,
}

impl Frame {
    pub fn new() -> Self {
        let size = size().unwrap(); 
        Frame {
            width: size.0,
            height: size.1,
            stdout: stdout(),
        }
    }

    pub fn area(&self) -> Rect { 
        Rect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }
    }
}

impl Frame{
    pub fn render_widget<T: Widget>(&mut self, widget: T, rect: Rect) {
        widget.render(&mut self.stdout, rect);
    }
}

pub enum Direction {
    Vertical, Horizontal
}

#[derive(Clone, Copy)]
pub enum Constraint {
    Length(usize),
    Percentage(usize),
    Fill(usize),
}

pub struct Layout {
    direction: Direction,
    constraints: ArrayBuilder<Constraint, 128>,
}

impl Layout {
    pub fn default() -> Self {
        Layout {
            direction: Direction::Horizontal,
            constraints: ArrayBuilder::<Constraint, 128>::new(),
        }
    }

    pub fn direction(self, direction: Direction) -> Self {
        Layout {
            direction: direction,
            constraints: self.constraints,
        }
    }

    pub fn constraints(self, constraints: &[Constraint]) -> Self {
        let mut a = ArrayBuilder::<Constraint, 128>::new();
        for constraint in constraints {
            a.push(*constraint);
        }
        Layout {
            direction: self.direction,
            constraints: a,
        }
    }

    pub fn split(&self, rect: Rect) -> ArrayBuilder<Rect, 512> {
        let mut rects = ArrayBuilder::new();
        let mut fillable_space = match self.direction { Direction::Horizontal => rect.width, Direction::Vertical => rect.height};
        let mut fillable_rects: u16 = 0;
        let total = fillable_space;
        for constraint in self.constraints.iter() {
            match constraint {
                Constraint::Fill(_) => fillable_rects += 1,
                Constraint::Length(l) => fillable_space -= *l as u16,
                Constraint::Percentage(p) => fillable_space -= (total*((*p) as u16))/100,
            };
        }
        if fillable_rects > 0 {
            fillable_space = fillable_space / fillable_rects;
        }
        let mut start = 0;
        for constraint in self.constraints.iter() {
            rects.push(match self.direction {
                Direction::Horizontal => {
                    let width = match constraint {
                        Constraint::Fill(_) => fillable_space,
                        Constraint::Length(l) => *l as u16,
                        Constraint::Percentage(p) => (total*((*p) as u16))/100,
                    };
                    let rect = Rect {
                        x: start,
                        y: rect.y,
                        width: width,
                        height: rect.height,
                    };
                    start += width;
                    rect
                },
                Direction::Vertical => {
                    let height = match constraint {
                        Constraint::Fill(_) => fillable_space,
                        Constraint::Length(l) => *l as u16,
                        Constraint::Percentage(p) => (total*((*p) as u16))/100,
                    };
                    let rect = Rect {
                        x: rect.x,
                        y: start,
                        width: rect.width,
                        height: height,
                    };
                    start += height;
                    rect
                }
            });
        }
        rects
    }
}

pub struct Margin {
    pub m: u16
}

#[derive(Clone, Copy)]
pub struct Rect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl Rect {
    pub fn inner(self, margin: Margin) -> Self {
        Rect {
            x: self.x + margin.m,
            y: self.y + margin.m,
            width: self.width - (margin.m * 2),
            height: self.height - (margin.m * 2),
        }
    }
}

pub trait Widget {
    fn render(&self, stdout: &mut Stdout, rect: Rect);
}

pub trait Renderer<T: Widget> {
    fn render_widget(&mut self, widget: T, rect: Rect);
}

pub struct Input {
    pub value: ArrayBuilder<u8, 128>
}

impl Input {
    pub fn new() -> Self {
        Input {
            value: ArrayBuilder::new(),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        panic!();
    }

    pub fn reset(&mut self) {
        self.value = ArrayBuilder::new();
    }

    pub fn value(&self) -> &str {
        panic!()
    }
}

pub enum Borders {ALL, NONE}

pub struct Block {
    borders: Borders
}

impl Block {
    pub fn default() -> Self {
        Block {borders: Borders::NONE}
    }

    pub fn borders(self, borders: Borders) -> Self {
        Block {
            borders: borders
        }
    }
}

impl Widget for Block {
    fn render(&self, stdout: &mut Stdout, rect: Rect) {
        stdout.queue(MoveTo(rect.x, rect.y));

        let mut horizontal_bar: [u16; 512] = [0x2500; 512];

        horizontal_bar[0] = 0x250C;
        horizontal_bar[(rect.width-1) as usize] = 0x2510;
        let mut utf8_buf = [0; 1024];
        let len = convert_to_u8_slice(&horizontal_bar[0..rect.width as usize], &mut utf8_buf);
        stdout.write(&utf8_buf[0..len]);

        for i in 1..rect.height-1 {
            stdout.queue(MoveTo(rect.x, rect.y+i));

            let mut utf8_buf = [0; 1024];
            let mut between_bar = [b' ' as u16; 512];
            between_bar[0] = 0x2502;
            between_bar[rect.width as usize -1] = 0x2502;
            let len = convert_to_u8_slice(&between_bar[0..rect.width as usize], &mut utf8_buf);

            stdout.write(&utf8_buf[0..len]);
        }

        horizontal_bar[0] = 0x2514;
        horizontal_bar[(rect.width-1) as usize] = 0x2518;
        let mut utf8_buf = [0; 1024];
        let len = convert_to_u8_slice(&horizontal_bar[0..rect.width as usize], &mut utf8_buf);
        stdout.queue(MoveTo(rect.x, rect.y+rect.height-1));
        stdout.write(&utf8_buf[0..len]);
    }
}

#[derive(Clone, Copy)]
pub enum Alignment {
    Left, Center, Right
}
































pub struct RawU16Buffer<const N: usize> {  
    pub buf: [u16; N]
}

impl<const N: usize> Widget for RawU16Buffer<N> {
    fn render(&self, stdout: &mut Stdout, rect: Rect) {
        panic!();
            // let mut sbuf: [u8; 4096] = [0; 4096];
            // let t = convert_to_u8_slice(&self.buf, &mut sbuf);
            // let s = unsafe{std::str::from_utf8_unchecked(&sbuf[0..t])};
            // let mut start = 0;
            // let mut index = 0;
            // let mut line = 0;
            // // let mut i = 0;
            // // let mut line = 0;
            // while index < t {
            //     if sbuf[index] == b'\n' {
            //         buf.set_string(rect.x, rect.y + line, std::str::from_utf8(&sbuf[start..index]).unwrap(), Style::new());
            //         line += 1;
            //         index += 1;
            //         start = index;
            //     } else {
            //         index += 1;
            //     }
            // }
            // if start < t {
            //     buf.set_string(rect.x, rect.y + line, std::str::from_utf8(&sbuf[start..t]).unwrap(), Style::new());
            // }
    }
}

pub fn convert_to_u8(char: u16) -> [u8; 3] {
	let w = ((char & 0b1111000000000000) >> 12) as u8;
	let x = ((char & 0b0000111100000000) >> 8) as u8;
	let y = ((char & 0b0000000011110000) >> 4) as u8;
	let z = (char & 0b0000000000001111) as u8;

	let byte_1 = 0b11100000 | w;
	let byte_2 = 0b10000000 | (x << 2) | (y >> 2);
	let byte_3 = 0b10000000 | ((y << 4) & 0b00110000) | z;
	return [byte_1, byte_2, byte_3];
}

pub fn convert_to_u8_slice(src: &[u16], dst: &mut [u8]) -> usize {
    let mut index = 0;
    for i in src {
        if i & 0xFF00 == 0x0000 {
            dst[index] = (i & 0x00FF) as u8;
            index += 1;
        } else {
            dst[index..index+3].copy_from_slice(&convert_to_u8(*i));
            index +=3;
        }
    }
    return index;
}

#[derive(Clone, Copy)]
pub struct FixedLengthString<const N: usize> {
    pub buf: [u8; N],
    pub length: usize,
    alignment: Alignment,
}

impl<const N: usize> FixedLengthString<N> {
    pub fn new(src: &[u8]) -> Self {
        let mut buf = [0; N];
        buf[0..src.len()].copy_from_slice(src);
        FixedLengthString { buf, length: src.len(), alignment: Alignment::Left }
    }

    pub fn alignment(self: Self, alignment: Alignment) -> Self {
        FixedLengthString { buf: self.buf, length: self.length, alignment: alignment }
    }
}

impl<const N: usize> Command for FixedLengthString<N> {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        f.write_str(std::str::from_utf8(&self.buf[0..self.length]).unwrap())
    }
}

impl<const N: usize> AsRef<str> for FixedLengthString<N> {
    fn as_ref(&self) -> &str {
        std::str::from_utf8(&self.buf[0..self.length]).unwrap()
    }
}

impl<const N: usize> Widget for FixedLengthString<N> {
    fn render(&self, stdout: &mut Stdout, rect: Rect)
    where
        Self: Sized {
        panic!();
            // buf.set_string(area.x, area.y, std::str::from_utf8(&self.buf[0..self.length]).unwrap(), Style::new());
    }
}

pub fn convert_4bit_to_hex_char(val: u8) -> u8 {
    match val {
        0x0 => b'0',
        0x1 => b'1',
        0x2 => b'2',
        0x3 => b'3',
        0x4 => b'4',
        0x5 => b'5',
        0x6 => b'6',
        0x7 => b'7',
        0x8 => b'8',
        0x9 => b'9',
        0xa => b'a',
        0xb => b'b',
        0xc => b'c',
        0xd => b'd',
        0xe => b'e',
        0xf => b'f',
        _ => panic!("Invalid"),
    }
}

pub fn convert_usize_to_u8_string(val: usize) -> FixedLengthString<16> {
    let size = size_of::<usize>();
    let mut string = [0; 32];
    let high_mask: u8 = 0xf0;
    let low_mask = 0x0f;
    for i in 0..size {
        string[i] = convert_4bit_to_hex_char(((val >> size-i-1) as u8 & high_mask) >> 4);
        string[i+1] = convert_4bit_to_hex_char((val >> size-i-1) as u8 & low_mask);
    }
    FixedLengthString::new(&string[0..size*2])
}
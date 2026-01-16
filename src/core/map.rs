use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex, RwLock, Weak}};

use crate::{core::{bitwise_operations::and_byte, board::{Board, BoardArrangement}, board_state::BoardState, piece::*}, headless, log};

pub struct PointerToBoard {
    pub ptr: Weak<RwLock<BoardArrangementPositions>>,
    pub index: usize,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BoardPieces {
    blackPawns: usize,
    blackRooks: usize,
    blackKnights: usize,
    blackBishops: usize,
    blackQueens: usize,
    blackKings: usize,

    whitePawns: usize,
    whiteRooks: usize,
    whiteKnights: usize,
    whiteBishops: usize,
    whiteQueens: usize,
    whiteKings: usize,
}

fn get_board_pieces(board: &Board) -> BoardPieces {
    let mut board_pieces = BoardPieces{
        blackPawns: 0,
        blackRooks: 0,
        blackKnights: 0,
        blackBishops: 0,
        blackQueens: 0,
        blackKings: 0,
        whitePawns: 0,
        whiteRooks: 0,
        whiteKnights: 0,
        whiteBishops: 0,
        whiteQueens: 0,
        whiteKings: 0,
    };

    let presense_board = and_byte(board.pieces, PRESENCE_BITS);
    let color_board = and_byte(board.pieces, COLOR_BITS);
    let type_board = and_byte(board.pieces, TYPE_BITS);

    for i in 0..64 {
        if presense_board[i] == PRESENT {
            let is_black = color_board[i] == BLACK;
            match type_board[i] {
                PAWN => {
                    if is_black {
                        board_pieces.blackPawns += 1;
                    } else {
                        board_pieces.whitePawns += 1;
                    }
                }
                ROOK => {
                    if is_black {
                        board_pieces.blackRooks += 1;
                    } else {
                        board_pieces.whiteRooks += 1;
                    }
                }
                KNIGHT => {
                    if is_black {
                        board_pieces.blackKnights += 1;
                    } else {
                        board_pieces.whiteKnights += 1;
                    }
                }
                BISHOP => {
                    if is_black {
                        board_pieces.blackBishops += 1;
                    } else {
                        board_pieces.whiteBishops += 1;
                    }
                }
                QUEEN => {
                    if is_black {
                        board_pieces.blackQueens += 1;
                    } else {
                        board_pieces.whiteQueens += 1;
                    }
                }
                KING => {
                    if is_black {
                        board_pieces.blackKings += 1;
                    } else {
                        board_pieces.whiteKings += 1;
                    }
                },
                _ => panic!("Invalid piece type"),
            }
        }
    }

    board_pieces
}

#[derive(Clone)]
pub struct Positions {
    pub map: Arc<RwLock<HashMap<
        BoardArrangement,
        Arc<RwLock<BoardArrangementPositions>>
    >>>
}

pub const PAGE_SIZE: usize = 4096 * 1024;
pub const PAGE_BOARD_COUNT: usize = PAGE_SIZE / size_of::<RwLock<BoardState>>();

pub struct BoardArrangementPositions {
    pub map: HashMap<Board, usize>,
    pub positions: [Option<Box<Vec<RwLock<BoardState>>>>; 128],
    pub size: usize,
}

impl BoardArrangementPositions {
    pub fn new() -> Self {
        BoardArrangementPositions {
            map: HashMap::new(),
            positions: std::array::from_fn(|_| { None }),
            size: 0,
        }
    }

    pub fn get(&self, index: usize) -> &RwLock<BoardState> {
        let page_number = index / PAGE_BOARD_COUNT;
        let page_index = index % PAGE_BOARD_COUNT;
        let page = self.positions.get(page_number).unwrap().as_ref().unwrap();
        match page.get(page_index) {
            Some(board_state) => board_state,
            None => {
                panic!("Page index out of bounds: {} for page number: {} with capacity: {}", page_index, page_number, page.capacity());
            }
        }
    }
}

pub enum Presence<T> {
    Present{value: T},
    Absent{value: T},
}

impl Positions {

    pub fn new() -> Self {
        Positions {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_board_arrangement_positions(&self, board: &Board) -> Arc<RwLock<BoardArrangementPositions>> {
        let readable_board_pieces_map = self.map.read().unwrap();
        let board_arrangement = board.get_board_arrangement();
        let board_pieces_map = readable_board_pieces_map.get(&board_arrangement);
        match board_pieces_map {
            Some(board_pieces_map) => board_pieces_map.clone(),
            None => {
                drop(readable_board_pieces_map);
                let mut writable_board_pieces_map = self.map.write().unwrap();
                match writable_board_pieces_map.get(&board_arrangement) {
                    Some(board_pieces_map) => board_pieces_map.clone(),
                    None => {
                        let new_board_pieces_map = Arc::new(RwLock::new(BoardArrangementPositions::new()));
                        writable_board_pieces_map.insert(board_arrangement, new_board_pieces_map.clone());
                        new_board_pieces_map
                    }
                }
            }
        }
    }

    // pub fn is_present(&self, board: &Board) -> bool {
    //     match self.map.read().unwrap().get(&get_board_pieces(board)) {
    //         Some(positions_map) => positions_map.read().unwrap().contains_key(board),
    //         None => false,
    //     }
    // }

    pub fn get(&self, board: &Board) -> Option<PointerToBoard> {
        let board_arrangement_positions = self.get_board_arrangement_positions(&board);
        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
        let index = readable_board_arrangement_positions.map.get(&board);
        match index {
            Some(index) => {
                Some(PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: *index })
            }
            None => None,
        }
    }

    pub fn edit<'a, 'b>(&'a self, board: &'b Board) -> Presence<PointerToBoard> {
        // log!("Editing board");
        let board_arrangement_positions = self.get_board_arrangement_positions(board);
        // log!("Got board pieces map");
        let readable_board_arrangement_positions = board_arrangement_positions.read().unwrap();
        // log!("Got readable positions map");
        let board_state_position = readable_board_arrangement_positions.map.get(&board);
        if board_state_position.is_some() {
            let index = *board_state_position.unwrap();
            Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
        } else {
            drop(readable_board_arrangement_positions);
            let mut writable_board_arrangement_positions = board_arrangement_positions.write().unwrap();
            let board_state_position = writable_board_arrangement_positions.map.get(&board);
            if board_state_position.is_some() {
                let index = *board_state_position.unwrap();
                Presence::Present { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            } else {
                let index = writable_board_arrangement_positions.size;
                let page = index / PAGE_BOARD_COUNT;
                let page_index = index % PAGE_BOARD_COUNT;
                if page_index == 0 {
                    let k = writable_board_arrangement_positions.positions.get_mut(page).unwrap();
                    *k = Some(Box::new(Vec::with_capacity(PAGE_BOARD_COUNT)));
                    // log!("Created new page");
                }
                let vec = writable_board_arrangement_positions.positions.get_mut(page).unwrap().as_mut().unwrap();
                vec.push(RwLock::new(BoardState::new()));
                writable_board_arrangement_positions.map.insert(*board, index);
                writable_board_arrangement_positions.size += 1;
                Presence::Absent { value: PointerToBoard { ptr: Arc::downgrade(&board_arrangement_positions), index: index } }
            }
        }
    }

    pub fn len(&self) -> String {
        let mut len = 0;
        for (_, value) in self.map.read().unwrap().iter() {
            let board_arrangement_positions = value.read().unwrap();
            len = len + board_arrangement_positions.size;
        }
        format!("{} {}", self.map.read().unwrap().len(), len)
    }
}
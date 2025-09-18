use std::{collections::{HashMap, HashSet}, sync::{Arc, RwLock}};

use crate::{core::{board::Board, board_state::BoardState, piece::*}, headless};

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

    for piece in board.pieces {
        if get_presence(piece) == PRESENT {
            let is_black = get_color(piece) == BLACK;
            match get_type(piece) {
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
        BoardPieces,
        Arc<RwLock<HashMap<Board, Arc<RwLock<BoardState>>>>>
    >>>
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

    pub fn get_board_pieces_map(&self, board: &Board) -> Arc<RwLock<HashMap<Board, Arc<RwLock<BoardState>>>>> {
        let readable_board_pieces_map = self.map.read().unwrap();
        let board_pieces = get_board_pieces(board);
        let board_pieces_map = readable_board_pieces_map.get(&board_pieces);
        match board_pieces_map {
            Some(board_pieces_map) => board_pieces_map.clone(),
            None => {
                drop(readable_board_pieces_map);
                let mut writable_board_pieces_map = self.map.write().unwrap();
                match writable_board_pieces_map.get(&board_pieces) {
                    Some(board_pieces_map) => board_pieces_map.clone(),
                    None => {
                        let new_board_pieces_map = Arc::new(RwLock::new(HashMap::new()));
                        writable_board_pieces_map.insert(board_pieces, new_board_pieces_map.clone());
                        new_board_pieces_map
                    }
                }
            }
        }
    }
    pub fn is_present(&self, board: &Board) -> bool {
        match self.map.read().unwrap().get(&get_board_pieces(board)) {
            Some(positions_map) => positions_map.read().unwrap().contains_key(board),
            None => false,
        }
    }

    pub fn get(&self, board: &Board) -> Option<Arc<RwLock<BoardState>>> {
        let board_pieces = get_board_pieces(board);
        let readable_board_pieces_map = self.map.read().unwrap();
        match readable_board_pieces_map.get(&board_pieces) {
            Some(positions_map) => match positions_map.read().unwrap().get(board) {
                Some(board_state) => Some(board_state.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn edit(&self, board: &Board) -> Presence<Arc<RwLock<BoardState>>> {
        headless!("Editing board");
        let positions_map = self.get_board_pieces_map(board);
        headless!("Got board pieces map");
        let readable_positions_map = positions_map.read().unwrap();
        headless!("Got readable positions map");
        let board_state = readable_positions_map.get(&board);
        if board_state.is_some() {
            Presence::Present { value: board_state.unwrap().clone() }
        } else {
            drop(readable_positions_map);
            let mut writable_positions_map = positions_map.write().unwrap();
            let new_board_state = Arc::new(RwLock::new(BoardState::new()));
            writable_positions_map.insert(*board, new_board_state.clone());
            Presence::Absent { value: new_board_state }
        }
    }

    pub fn keys(&self) -> HashSet<Board> {
        let mut keys = HashSet::new();
        let binding = self.map.clone();
        let readable_map = binding.read().unwrap();
        for (_, value) in readable_map.clone().into_iter() {
            let sub_keys = value.read().unwrap().keys().cloned().collect::<HashSet<Board>>();
            keys.extend(sub_keys);
        }
        keys
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        let binding = self.map.clone();
        let readable_map = binding.read().unwrap();
        for (_, value) in readable_map.clone().into_iter() {
            len = len + value.read().unwrap().len();
        }
        len
    }

    pub fn remove_keys(&self, board: Vec<Board>) {
        let writable_map = self.map.write().unwrap();
        for board in board {
            let board_pieces = get_board_pieces(&board);
            let board_pieces_map = writable_map.get(&board_pieces);
            if let Some(board_pieces_map) = board_pieces_map {
                let mut writable_board_pieces_map = board_pieces_map.write().unwrap();
                writable_board_pieces_map.remove(&board);
            }
        }
    }
}
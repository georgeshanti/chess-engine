use std::sync::Arc;
use std::sync::RwLock;

use crate::core::piece::*;
use crate::core::board_state::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Board {
    pub pieces: [u8; 64],
}

const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const KNIGHT_DIRECTIONS: [(i8, i8); 8] = [(2, 1), (2, -1), (-2, 1), (-2, -1), (1, 2), (1, -2), (-1, 2), (-1, -2)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
const QUEEN_DIRECTIONS: [(i8, i8); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];

trait Coordinates<T> {
    fn multiply(self: &Self, multiplier: i8) -> Self;
    fn add(self: &Self, other: Self) -> Self;
    fn as_usize(self: &Self) -> (usize, usize);
}

impl Coordinates<(i8, i8)> for (i8, i8) {
    fn multiply(self: &Self, multiplier: i8) -> Self {
        (self.0 * multiplier, self.1 * multiplier)
    }

    fn add(self: &Self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1)
    }

    fn as_usize(self: &Self) -> (usize, usize) {
        (self.0 as usize, self.1 as usize)
    }
}

impl Board {
    pub fn new() -> Self {
        Self { pieces: [0; 64] }
    }

    pub fn get(self: &Self, rank: usize, file: usize) -> u8 {
        self.pieces[rank*8+file]
    }

    pub fn set(self: &mut Self, rank: usize, file: usize, piece: u8) {
        self.pieces[rank*8+file] = piece
    }

    pub fn clone(self: &Self) -> Self {
        let mut new_board = Board::new();
        new_board.pieces = self.pieces.clone();
        new_board
    }

    pub fn inverted(self: &Self) -> Self {
        let mut new_board = Board::new();
        for i in 0..64 {
            if get_presence(self.pieces[i]) == PRESENT {
                new_board.pieces[63-i] = negate_color(self.pieces[i]);
            } else {
                new_board.pieces[63-i] = 0b0;
            }
        }
        return new_board;
    }

    pub fn normalize_opponent_pieces(self: &mut Self) {
        for i in 0..64 {
            if self.pieces[i] == PRESENT | BLACK | PAWN | HAS_MOVED_TWO_SQUARES {
                self.pieces[i] = PRESENT | BLACK | PAWN | HAS_NOT_MOVED_TWO_SQUARES
            }
        }
    }

    pub fn find_moves(self: &Self) -> Vec<Board> {
        let mut moves = Vec::new();
        for i in 0..64 {
            let piece = self.pieces[i];
            if get_presence(piece) == EMPTY || get_color(piece) == BLACK {
                continue;
            }
            let rank = i / 8;
            let file = i % 8;
            let source_piece = self.pieces[i];
            match get_type(source_piece) {
                PAWN => {
                    if get_presence(self.get(rank+1, file)) == EMPTY {
                        let mut new_board = self.clone();
                        new_board.set(rank+1, file, PRESENT | WHITE | PAWN | HAS_NOT_MOVED_TWO_SQUARES);
                        moves.push(new_board);

                        if get_presence(self.get(rank+2, file)) == EMPTY {
                            let mut new_board = self.clone();
                            new_board.set(rank+2, file, PRESENT | WHITE | PAWN | HAS_MOVED_TWO_SQUARES);
                            moves.push(new_board);
                        }
                    }
                },
                ROOK | BISHOP | QUEEN | KNIGHT | KING => {
                    let max_distance: i8 = match get_type(source_piece) {
                        ROOK | BISHOP | QUEEN => 8,
                        KNIGHT | KING => 2,
                        _ => panic!("Not a valid type")
                    };
                    let directions = match get_type(source_piece) {
                        ROOK => &ROOK_DIRECTIONS[..],
                        BISHOP => &BISHOP_DIRECTIONS[..],
                        QUEEN | KING => &QUEEN_DIRECTIONS[..],
                        KNIGHT => &KNIGHT_DIRECTIONS[..],
                        _ => panic!("Not a valid type")
                    };
                    let mut can_move_in_directions = vec![true; directions.len()];
                    for distance in 1..max_distance {
                        for direction_idx in 0..directions.len() {
                            let direction = directions[direction_idx];
                            if can_move_in_directions[direction_idx] {
                                let destination: (i8, i8) = direction.multiply(distance).add((rank as i8, file as i8));
                                if (0<=destination.0) && (destination.0<8) && 0<=destination.1 && destination.1<8 {
                                    let destination = destination.as_usize();
                                    let piece = self.get(destination.0, destination.1);
                                    if get_presence(piece) == EMPTY {
                                        let mut new_board = self.clone();
                                        new_board.set(destination.0, destination.1, PRESENT | WHITE | ROOK);
                                        moves.push(new_board);
                                    } else {
                                        can_move_in_directions[direction_idx] = false;
                                        if get_color(piece) == BLACK {
                                            let mut new_board = self.clone();
                                            new_board.set(destination.0, destination.1, PRESENT | WHITE | ROOK);
                                            moves.push(new_board);
                                        }
                                    }
                                } else {
                                    can_move_in_directions[direction_idx] = false;
                                }
                            }
                        }
                    }
                },
                _ => panic!("Invalid piece type"),
            }
        }
        moves.iter().map(|&board| {
            let mut new_board = board.clone();
            new_board.inverted();
            new_board.normalize_opponent_pieces();
            new_board
        }).collect()
    }

    fn is_opponent_in_check(self: &Self) -> bool {
        let mut king_rank: i8 = 8;
        let mut king_file: i8 = 8;
        for rank in 0..8 {
            for file in 0..8 {
                let piece = self.get(rank, file);
                if get_presence(piece) == PRESENT && get_color(piece) == BLACK && get_type(piece) == KING {
                    king_rank = rank as i8;
                    king_file = file as i8;
                    break;
                }
            }
        }
        if king_rank == 8 || king_file == 8 {
            return true;
        }

        for direction in QUEEN_DIRECTIONS {
            for distance in 1..8 {
                let destination: (i8, i8) = direction.multiply(distance).add((king_rank, king_file));
                if direction.0 == 0 || direction.1 == 0 {
                    if (0<=destination.0) && (destination.0<8) && (0<=destination.1) && (destination.1<8) {
                        let destination = destination.as_usize();
                        let piece = self.get(destination.0, destination.1);
                        if get_presence(piece) == PRESENT {
                            if get_color(piece) == WHITE {
                                return true;
                            } else {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        return false;
    }

    pub fn get_evaluation(self: &Self) -> (Evaluation, Vec<Board>) {
        let moves = self.find_moves();
        let mut legal_moves: Vec<Board> = vec![];
        for board in moves.iter() {
            if !board.is_opponent_in_check() {
                legal_moves.push(*board);
            }
        }
        if legal_moves.len() == 0 {
            let inverted_board = self.inverted();
            if inverted_board.is_opponent_in_check() {
                return (Evaluation{
                        result: PositionResult::Loss,
                        score: 0,
                    },
                    vec![],
                );
            } else {
                return (
                    Evaluation{
                        result: PositionResult::Draw,
                        score: 0,
                    },
                    vec![],
                );
            }
        }
        let mut material = 0;
        for i in 0..64 {
            let piece = self.pieces[i];
            if get_presence(piece) == PRESENT {
                let multiplier: i8 = match get_color(piece) {
                    WHITE => 1,
                    BLACK => -1,
                    _ => panic!("Invalid color"),
                };
                material += get_material_value(piece) * multiplier as i64;
            }
        }
        return (
            Evaluation {
                result: PositionResult::Scored,
                score: material as i32,
            },
            legal_moves,
        );
    }
}
use std::fmt::Display;

use crate::core::bitwise_operations::and_byte;
use crate::core::piece::*;
use crate::core::board_state::*;
use crate::log;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Board {
    pub pieces: [u8; 64],
}

const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const KNIGHT_DIRECTIONS: [(i8, i8); 8] = [(2, 1), (2, -1), (-2, 1), (-2, -1), (1, 2), (1, -2), (-1, 2), (-1, -2)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
const QUEEN_DIRECTIONS: [(i8, i8); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];
const PAWN_DIAGONALS: [(i8, i8); 2] = [(1, 1), (1, -1)];

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
        // Board{pieces: xor_byte(self.pieces, COLOR_BITS)}
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

    #[inline(never)]
    pub fn find_moves(self: &Self) -> Vec<Board> {
        let presence_board = Board {pieces: and_byte(self.pieces, PRESENCE_BITS)};
        let color_board = Board {pieces: and_byte(self.pieces, COLOR_BITS)};
        let type_board = Board {pieces: and_byte(self.pieces, TYPE_BITS)};
        let mut vec_length: usize = 0;
        for i in 0..64 {
            if presence_board.pieces[i] == EMPTY || color_board.pieces[i] == BLACK {
                continue;
            }
            vec_length +=  get_max_movement(type_board.pieces[i]);
        }
        let mut moves = Vec::with_capacity(vec_length);
        for i in 0..64 {
            if presence_board.pieces[i] == EMPTY || color_board.pieces[i] == BLACK {
                continue;
            }
            // if get_presence(piece) == EMPTY || get_color(piece) == BLACK {
            //     continue;
            // }
            let rank = i / 8;
            let file = i % 8;
            // match type_board {
            //     Some(_) => {}
            //     None => {
            //         type_board = Some(Board {pieces: and_byte(self.pieces, TYPE_BITS)});
            //     }
            // };
            match type_board.pieces[i] {
            // match get_type(source_piece) {
                PAWN => {
                    if rank<7 && presence_board.get(rank+1, file) == EMPTY {
                        let mut new_board = self.clone();
                        new_board.set(rank, file, EMPTY);
                        new_board.set(rank+1, file, PRESENT | WHITE | PAWN | HAS_NOT_MOVED_TWO_SQUARES);
                        moves.push(new_board);

                        if rank<6 && presence_board.get(rank+2, file) == EMPTY {
                            let mut new_board = self.clone();
                            new_board.set(rank, file, EMPTY);
                            new_board.set(rank+2, file, PRESENT | WHITE | PAWN | HAS_MOVED_TWO_SQUARES);
                            moves.push(new_board);
                        }
                    }
                    for diagonal in PAWN_DIAGONALS {
                        let destination: (i8, i8) = diagonal.add((rank as i8, file as i8));
                        if (0<=destination.0) && (destination.0<8) && 0<=destination.1 && destination.1<8 {
                            let destination = destination.as_usize();
                            let target_piece_presence = presence_board.get(destination.0, destination.1);
                            let target_piece_color = color_board.get(destination.0, destination.1);
                            if target_piece_presence == PRESENT && target_piece_color == BLACK {
                                let mut new_board = self.clone();
                                new_board.set(rank, file, EMPTY);
                                new_board.set(destination.0, destination.1, PRESENT | WHITE | PAWN | HAS_NOT_MOVED_TWO_SQUARES);
                                moves.push(new_board);
                            }
                        }
                    }
                },
                ROOK | BISHOP | QUEEN | KNIGHT | KING => {
                    let max_distance: i8 = match type_board.pieces[i] {
                        ROOK | BISHOP | QUEEN => 8,
                        KNIGHT | KING => 2,
                        _ => panic!("Not a valid type")
                    };
                    let directions = match type_board.pieces[i] {
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
                                    let piece_presence = presence_board.get(destination.0, destination.1);
                                    let piece_color = color_board.get(destination.0, destination.1);
                                    if piece_presence == EMPTY {
                                        if type_board.pieces[i] == KNIGHT {
                                        }
                                        let mut new_board = self.clone();
                                        new_board.set(rank, file, EMPTY);
                                        new_board.set(destination.0, destination.1, PRESENT | WHITE | type_board.pieces[i]);
                                        moves.push(new_board);
                                    } else {
                                        can_move_in_directions[direction_idx] = false;
                                        if piece_color == BLACK {
                                            if type_board.pieces[i] == KNIGHT {
                                            }
                                            let mut new_board = self.clone();
                                            new_board.set(rank, file, EMPTY);
                                            new_board.set(destination.0, destination.1, PRESENT | WHITE | type_board.pieces[i]);
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
            new_board = new_board.inverted();
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
                                if ROOK_DIRECTIONS.contains(&direction) && get_type(piece) == ROOK {
                                    return true;
                                } else if BISHOP_DIRECTIONS.contains(&direction) && get_type(piece) == BISHOP {
                                    return true;
                                } else if get_type(piece) == QUEEN {
                                    return true;
                                }
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

    pub fn get_evaluation(self: &Self) -> (Evaluation, Box<[Board]>) {
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
                    Box::new([]),
                );
            } else {
                return (
                    Evaluation{
                        result: PositionResult::Draw,
                        score: 0,
                    },
                    Box::new([]),
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
            legal_moves.into_boxed_slice(),
        );
    }

    // fn add_piece_to_pawns(
    //     white_pawns: &mut u64,
    //     white_major: &mut [u8; 6],
    //     black_pawns: &mut u64,
    //     black_major: &mut [u8; 6],
    // )

    pub fn get_board_arrangement(self: &Self) -> BoardArrangement {
        let mut white_pawns: u64 = 0;
        let mut white_major: [u8; 6] = [0; 6];
        let mut black_pawns: u64 = 0;
        let mut black_major: [u8; 6] = [0; 6];
        for i in 0..64 {
            let piece = self.pieces[i];
            if get_presence(piece) == EMPTY {
                continue;
            } else {
                let piece_type = get_type(piece);
                if piece_type == PAWN {
                    if get_color(piece) == WHITE {
                        white_pawns = white_pawns | (1 << i);
                    } else {
                        black_pawns = black_pawns | (1 << 63-i);
                    }
                }
                let index = (piece_type >> 3) - 1;
                if get_color(piece) == WHITE {
                    white_major[index as usize] = white_major[index as usize] + 1;
                } else {
                    black_major[index as usize] = black_major[index as usize] + 1;
                }
            }
        }
        return match white_pawns.cmp(&black_pawns) {
            std::cmp::Ordering::Greater => BoardArrangement {
                higher: PieceArrangement {
                    pawns: white_pawns,
                    major_pieces: white_major,
                },
                lower: PieceArrangement {
                    pawns: black_pawns,
                    major_pieces: black_major,
                },
            },
            std::cmp::Ordering::Less => BoardArrangement {
                higher: PieceArrangement {
                    pawns: black_pawns,
                    major_pieces: black_major,
                },
                lower: PieceArrangement {
                    pawns: white_pawns,
                    major_pieces: white_major,
                },
            },
            std::cmp::Ordering::Equal => match compare_u8_6(&white_major, &black_major) {
                std::cmp::Ordering::Greater => BoardArrangement {
                    higher: PieceArrangement {
                        pawns: white_pawns,
                        major_pieces: white_major,
                    },
                    lower: PieceArrangement {
                        pawns: black_pawns,
                        major_pieces: black_major,
                    },
                },
                std::cmp::Ordering::Less => BoardArrangement {
                    higher: PieceArrangement {
                        pawns: black_pawns,
                        major_pieces: black_major,
                    },
                    lower: PieceArrangement {
                        pawns: white_pawns,
                        major_pieces: white_major,
                    },
                },
                std::cmp::Ordering::Equal => BoardArrangement {
                    higher: PieceArrangement {
                        pawns: white_pawns,
                        major_pieces: white_major,
                    },
                    lower: PieceArrangement {
                        pawns: black_pawns,
                        major_pieces: black_major,
                    },
                },
            },
        }
    }
}

pub fn can_come_after(source: &PieceArrangement, destination: &PieceArrangement) -> bool {
    let mut extra_pieces = 0;
    for i in 0..6 {
        if i == (PAWN >> 3) as usize {
            continue;
        }
        if destination.major_pieces[i] > source.major_pieces[i] {
            extra_pieces += destination.major_pieces[i] - source.major_pieces[i];
        }
    }
    if destination.major_pieces[(PAWN >> 3) as usize - 1] > source.major_pieces[(PAWN >> 3) as usize - 1] {
        return false;
    }
    let missing_pawns = source.major_pieces[(PAWN >> 3) as usize - 1] - destination.major_pieces[(PAWN >> 3) as usize - 1];
    if missing_pawns < extra_pieces {
        return false;
    }
    let source_pawns = convert_u64_pawns_to_pawn_position_vector(source.pawns);
    let destination_pawns = convert_u64_pawns_to_pawn_position_vector(destination.pawns);
    return match_pawns(source_pawns, destination_pawns);
}

fn match_pawns(source: Vec<u8>, destination: Vec<u8>) -> bool {
    for destination_index in 0..destination.len() {
        let destination_pawn = destination[destination_index];
        for source_index in 0..source.len() {
            let source_pawn = source[source_index];
            let destination_pawn_rank = destination_pawn / 8;
            let destination_pawn_file = destination_pawn % 8;
            let source_pawn_rank = source_pawn / 8;
            let source_pawn_file = source_pawn % 8;

            if source_pawn_rank > destination_pawn_rank {
                continue;
            }
            let rank_difference = destination_pawn_rank - source_pawn_rank;
            let file_start = (source_pawn_file as i16 - rank_difference as i16).clamp(0, 7) as u8;
            let file_end = (source_pawn_file as i16 + rank_difference as i16).clamp(0, 7) as u8;
            if file_start <= destination_pawn_file && destination_pawn_file <= file_end {
                let mut new_destination = destination.clone();
                new_destination.remove(destination_index);
                if new_destination.len() == 0 {
                    return true;
                }
                let mut new_source = source.clone();
                new_source.remove(source_index);
                return match_pawns(new_source, new_destination);
            }
        }
    }
    return false;
}



fn convert_u64_pawns_to_pawn_position_vector(pawns: u64) -> Vec<u8> {
    let mut bool_pawns = Vec::new();
    for i in 0..64 {
        if (pawns & (1 << i)) != 0 {
            bool_pawns.push(i);
        }
    }
    bool_pawns
}

fn compare_u8_6(a: &[u8; 6], b: &[u8; 6]) -> std::cmp::Ordering {
    for i in 0..6 {
        if a[i] > b[i] {
            return std::cmp::Ordering::Greater;
        } else if a[i] < b[i] {
            return std::cmp::Ordering::Less;
        }
    }
    return std::cmp::Ordering::Equal;
}

#[derive(Eq, PartialEq, Hash, Clone, Serialize, Deserialize, Debug)]
pub struct BoardArrangement {
    higher: PieceArrangement,
    lower: PieceArrangement,
}

pub fn can_come_after_board_arrangement(source: &BoardArrangement, destination: &BoardArrangement) -> bool {
    if can_come_after(&source.higher, &destination.higher) && can_come_after(&source.lower, &destination.lower) {
        return true;
    }
    if can_come_after(&source.lower, &destination.higher) && can_come_after(&source.higher, &destination.lower) {
        return true;
    }
    return false;
}

impl Display for BoardArrangement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut board = Board { pieces: [0; 64] };
        for i in 0..64 {
            if 1<<i & self.higher.pawns != 0 {
                board.set(i / 8, i % 8, PRESENT | WHITE | PAWN | HAS_NOT_MOVED_TWO_SQUARES);
            }
            if 1<<i & self.lower.pawns != 0 {
                board.set((63-i) / 8, (63-i) % 8, PRESENT | BLACK | PAWN | HAS_NOT_MOVED_TWO_SQUARES);
            }
        }
        board.fmt(f)
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Serialize, Deserialize, Debug)]
pub struct PieceArrangement {
    pawns: u64,
    major_pieces: [u8; 6],
}

impl Display for Board {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let top_string = {
            let mut top_string = [0 as u16; 33];
            top_string[0] = 0x250C;
            for i in 0..7 {
                top_string[1+(4*i)] = 0x2500;
                top_string[1+(4*i)+1] = 0x2500;
                top_string[1+(4*i)+2] = 0x2500;
                top_string[1+(4*i)+3] = 0x252C;
            }
            top_string[29] = 0x2500;
            top_string[30] = 0x2500;
            top_string[31] = 0x2500;
            top_string[32] = 0x2510;

            String::from_utf16(&top_string).unwrap()
        };

        let between_string = {
            let mut between_string = [0 as u16; 33];
            between_string[0] = 0x251C;

            for i in 0..7 {
                between_string[1+(4*i)] = 0x2500;
                between_string[1+(4*i)+1] = 0x2500;
                between_string[1+(4*i)+2] = 0x2500;
                between_string[1+(4*i)+3] = 0x253C;
            }
            between_string[29] = 0x2500;
            between_string[30] = 0x2500;
            between_string[31] = 0x2500;
            between_string[32] = 0x2524;

            String::from_utf16(&between_string).unwrap()
        };

        let bottom_string = {
            let mut bottom_string = [0 as u16; 33];
            bottom_string[0] = 0x2514;

            for i in 0..7 {
                bottom_string[1+(4*i)] = 0x2500;
                bottom_string[1+(4*i)+1] = 0x2500;
                bottom_string[1+(4*i)+2] = 0x2500;
                bottom_string[1+(4*i)+3] = 0x2534;
            }
            bottom_string[29] = 0x2500;
            bottom_string[30] = 0x2500;
            bottom_string[31] = 0x2500;
            bottom_string[32] = 0x2518;

            String::from_utf16(&bottom_string).unwrap()
        };

        let mut message = String::from("");
        message += &top_string;
        message += "\n";

        for i in 0..8 {
            let mut row_chars = String::from_utf16(&[0x2502]).unwrap();
            for j in 0..8 {
                row_chars += " ";
                row_chars += &char(self.get(7-i, j));
                row_chars += " ";
                row_chars += &String::from_utf16(&[0x2502]).unwrap();
            }
            message += &row_chars;
            message += "\n";
            if(i!=7) {
                message += &between_string;
                message += "\n";
            }
        }
        message += &bottom_string;

        write!(f, "{}", message)

    }
}
pub const PRESENCE_BITS: u8 = 0b1 << 7;
pub const EMPTY: u8 = 0b0 << 7;
pub const PRESENT: u8 = 0b1 << 7;

pub fn get_presence(b: u8) -> u8 {
    b & PRESENCE_BITS
}

pub const COLOR_BITS: u8 = 0b1 << 6;
pub const WHITE: u8 = 0b0 << 6;
pub const BLACK: u8 = 0b1 << 6;

pub fn get_color(b: u8) -> u8 {
    b & COLOR_BITS
}

pub fn negate_color(b: u8) -> u8 {
    b ^ COLOR_BITS
}

pub const TYPE_BITS: u8 = 0b111 << 3;
pub const PAWN: u8 = 0b001 << 3;
pub const ROOK: u8 = 0b010 << 3;
pub const KNIGHT: u8 = 0b011 << 3;
pub const BISHOP: u8 = 0b100 << 3;
pub const QUEEN: u8 = 0b101 << 3;
pub const KING: u8 = 0b110 << 3;

pub fn get_type(b: u8) -> u8 {
    b & TYPE_BITS
}

pub fn get_type_string(b: u8) -> String {
    match get_type(b) {
        PAWN => "P",
        ROOK => "R",
        KNIGHT => "N",
        BISHOP => "B",
        QUEEN => "Q",
        KING => "K",
        _ => panic!("Invalid piece type"),
    }.to_string()
}

const MAJOR_PIECES: [u8; 4] = [ROOK, KNIGHT, BISHOP, QUEEN];

const HAS_MOVED_BITS: u8 = 0b1 << 2;
pub const HAS_MOVED: u8 = 0b1 << 2;
pub const HAS_NOT_MOVED: u8 = 0b0 << 2;

pub fn get_has_moved(b: u8) -> bool {
    (b & HAS_MOVED_BITS) == HAS_MOVED
}

const HAS_MOVED_TWO_SQUARES_BITS: u8 = 0b1 << 2;
pub const HAS_MOVED_TWO_SQUARES: u8 = 0b1 << 2;
pub const HAS_NOT_MOVED_TWO_SQUARES: u8 = 0b0 << 2;

pub fn get_has_moved_two_squares(b: u8) -> bool {
    (b & HAS_MOVED_TWO_SQUARES_BITS) == HAS_MOVED_TWO_SQUARES
}

pub fn char(b: u8) -> String {
	if get_presence(b) == EMPTY {
		return " ".to_string();
	}
	let mut char = 0x2600;
	match get_color(b) {
		WHITE => char += 0x005A,
		BLACK => char += 0x0054,
        _ => (),
	}
	match get_type(b) {
        PAWN => char += 0x0005,
        ROOK => char += 0x0002,
        KNIGHT => char += 0x0004,
        BISHOP => char += 0x0003,
        QUEEN => char += 0x0001,
        KING => char += 0x0000,
        _ => panic!("Invalid piece type"),
	}
	return String::from_utf16(&[char]).unwrap();
}

pub fn get_material_value(b: u8) -> i64 {
	match get_type(b) {
		PAWN => 1,
		ROOK => 5,
		KNIGHT => 3,
		BISHOP => 3,
		QUEEN => 9,
		KING => 0,
        _ => panic!("Invalid piece type"),
	}
}

pub fn get_max_movement(b: u8) -> usize {
	match get_type(b) {
		PAWN => 4,
		ROOK => 14,
		KNIGHT => 8,
		BISHOP => 13,
		QUEEN => 27,
		KING => 8,
        _ => panic!("Invalid piece type"),
	}
}
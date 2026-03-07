pub const COLOR_BITS: u8 = 0b1 << 3;
pub const WHITE: u8 = 0b0 << 3;
pub const BLACK: u8 = 0b1 << 3;

pub fn get_color(b: u8) -> u8 {
    b & COLOR_BITS
}

pub fn negate_color(b: u8) -> u8 {
    b ^ COLOR_BITS
}

pub const TYPE_BITS: u8 = 0b111;
pub const EMPTY: u8 = 0b000;
pub const PAWN: u8 = 0b001;
pub const PAWN_TWO: u8 = 0b010;
pub const ROOK: u8 = 0b011;
pub const KNIGHT: u8 = 0b100;
pub const BISHOP: u8 = 0b101;
pub const QUEEN: u8 = 0b110;
pub const KING: u8 = 0b111;

pub fn get_type(b: u8) -> u8 {
    b & TYPE_BITS
}

const MAJOR_PIECES: [u8; 4] = [ROOK, KNIGHT, BISHOP, QUEEN];

pub fn char(b: u8) -> String {
	if get_type(b) == EMPTY {
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
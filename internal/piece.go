package internal

const PresenceBits byte = 0b1 << 7
const Empty byte = 0b0 << 7
const Present byte = 0b1 << 7

func GetPresence(b byte) byte {
	return b & PresenceBits
}

const ColorBits byte = 0b1 << 6
const White byte = 0b0 << 6
const Black byte = 0b1 << 6

func GetColor(b byte) byte {
	return b & ColorBits
}

const TypeBits byte = 0b111 << 3
const Pawn byte = 0b001 << 3
const Rook byte = 0b010 << 3
const Knight byte = 0b011 << 3
const Bishop byte = 0b100 << 3
const Queen byte = 0b101 << 3
const King byte = 0b110 << 3

func GetType(b byte) byte {
	return b & TypeBits
}

var MajorPieces = []byte{Rook, Knight, Bishop, Queen}

const HasMovedBits byte = 0b1 << 2
const HasMoved byte = 0b1 << 2
const HasNotMoved byte = 0b0 << 2

func GetHasMoved(b byte) bool {
	return b&HasMovedBits == HasMoved
}

const HasMovedTwoSquaresBits byte = 0b1 << 2
const HasMovedTwoSquares byte = 0b1 << 2
const HasNotMovedTwoSquares byte = 0b0 << 2

func GetHasMovedTwoSquares(b byte) bool {
	return b&HasMovedTwoSquaresBits == HasMovedTwoSquares
}

func NegateColor(b byte) byte {
	return b ^ ColorBits
}

func Negate(b byte) byte {
	return b ^ 0b11111111
}

func EmptyBits(b byte, bits byte) byte {
	return b & Negate(bits)
}

func Char(b byte) string {
	if GetPresence(b) == Empty {
		return " "
	}
	char := 0x2600
	switch GetColor(b) {
	case White:
		char += 0x005A
	case Black:
		char += 0x0054
	}
	switch GetType(b) {
	case Pawn:
		char += 0x0005
	case Rook:
		char += 0x0002
	case Knight:
		char += 0x0004
	case Bishop:
		char += 0x0003
	case Queen:
		char += 0x0001
	case King:
		char += 0x0000
	}
	return string(char)
}

func GetMaterialValue(b byte) int {
	switch GetType(b) {
	case Pawn:
		return 1
	case Rook:
		return 5
	case Knight:
		return 3
	case Bishop:
		return 3
	case Queen:
		return 9
	case King:
		return 0
	}
	return 0
}

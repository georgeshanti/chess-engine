package internal

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

var squareLengthArray = [8]int{0, 1, 2, 3, 4, 5, 6, 7}
var moveLengthArray = [7]int{1, 2, 3, 4, 5, 6, 7}

type Board struct {
	NextTurn byte
	Pieces   [64]byte
}

var EmptyBoard = Board{}

func (b *Board) Get(rank int, file int) byte {
	return b.Pieces[rank*8+file]
}

func (b *Board) Set(rank int, file int, piece byte) {
	b.Pieces[rank*8+file] = piece
}

func (b *Board) DuplicateForNextMove() Board {
	var pieces [64]byte = [64]byte{}
	copy(pieces[:], b.Pieces[:])
	return Board{
		NextTurn: b.NextTurn ^ ColorBits,
		Pieces:   pieces,
	}
}

func (b *Board) NormalizeOpponentPieces() {
	for i, piece := range b.Pieces {
		if piece == Present|NegateColor(b.NextTurn)|Pawn|HasMovedTwoSquares {
			b.Pieces[i] = Present | NegateColor(b.NextTurn) | Pawn | HasNotMovedTwoSquares
		}
	}
}

func (b *Board) String() string {
	var top string = fmt.Sprintf("%s%s%s", string(0x250C), strings.Repeat(strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x252C)}, ""), 7), strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x2510)}, ""))
	var between string = fmt.Sprintf("%s%s%s", string(0x251C), strings.Repeat(strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x253C)}, ""), 7), strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x2524)}, ""))
	var bottom string = fmt.Sprintf("%s%s%s", string(0x2514), strings.Repeat(strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x2534)}, ""), 7), strings.Join([]string{string(0x2500), string(0x2500), string(0x2500), string(0x2518)}, ""))
	var rows []string
	for i := range squareLengthArray {
		rowchars := []string{}
		for j := range squareLengthArray {
			rowchars = append(rowchars, Char(b.Get(i, j)))
		}
		row := fmt.Sprintf("%s %s %s", string(0x2502), strings.Join(rowchars, fmt.Sprintf(" %s ", string(0x2502))), string(0x2502))
		rows = append(rows, row)
	}
	nextTurn := "White"
	if b.NextTurn == Black {
		nextTurn = "Black"
	}
	return fmt.Sprintf("Next turn: %s\n%s\n%s\n%s", nextTurn, top, strings.Join(rows, fmt.Sprintf("\n%s\n", between)), bottom)
}

func (b *Board) FindMoves() []Board {
	moves := []Board{}
	for rank := range squareLengthArray {
		for file := range squareLengthArray {
			piece := b.Get(rank, file)
			if GetColor(piece) != b.NextTurn {
				continue
			}
			switch GetType(piece) {
			case Pawn:
				{
					var direction int
					switch GetColor(piece) {
					case White:
						direction = -1
					case Black:
						direction = 1
					}
					if rank+direction < 0 || rank+direction > 7 {
						continue
					}
					if GetPresence(b.Get(rank+direction, file)) == Empty {
						var seventhRank int
						var secondRank int
						switch GetColor(piece) {
						case White:
							seventhRank = 1
							secondRank = 6
						case Black:
							seventhRank = 6
							secondRank = 1
						}
						if rank == seventhRank {
							for _, majorPiece := range MajorPieces {
								newPiece := Present | b.NextTurn | majorPiece
								if majorPiece == Rook {
									newPiece |= HasMoved
								}

								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+direction, file, newPiece)

								moves = append(moves, newBoard)
							}
						} else {
							newBoard := b.DuplicateForNextMove()
							newBoard.Set(rank, file, Empty)
							newBoard.Set(rank+direction, file, piece)

							moves = append(moves, newBoard)
							if rank == secondRank && GetPresence(b.Get(rank+(direction*2), file)) == Empty {
								newDoubleBoard := b.DuplicateForNextMove()
								newDoubleBoard.Set(rank, file, Empty)
								newDoubleBoard.Set(rank+(direction*2), file, piece|HasMovedTwoSquares)
								moves = append(moves, newDoubleBoard)
							}
						}
					}
					if file > 0 && GetPresence(b.Get(rank+direction, file-1)) == Present && GetColor(b.Get(rank+direction, file-1)) != b.NextTurn {
						newBoard := b.DuplicateForNextMove()
						newBoard.Set(rank, file, Empty)
						newBoard.Set(rank+direction, file-1, piece)
						moves = append(moves, newBoard)
					}
					if file < 7 && GetPresence(b.Get(rank+direction, file+1)) == Present && GetColor(b.Get(rank+direction, file+1)) != b.NextTurn {
						newBoard := b.DuplicateForNextMove()
						newBoard.Set(rank, file, Empty)
						newBoard.Set(rank+direction, file+1, piece)
						moves = append(moves, newBoard)
					}
				}
			case Rook:
				{
					var canMoveUpward bool = rank > 0
					var canMoveDownward bool = rank < 7
					var canMoveLeft bool = file > 0
					var canMoveRight bool = file < 7
					for _, i := range moveLengthArray {
						if canMoveUpward && rank-i >= 0 {
							if GetPresence(b.Get(rank-i, file)) == Present {
								canMoveUpward = false
							}
							if GetPresence(b.Get(rank-i, file)) == Empty || GetColor(b.Get(rank-i, file)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file, Present|b.NextTurn|Rook|HasMoved)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownward && rank+i <= 7 {
							if GetPresence(b.Get(rank+i, file)) == Present {
								canMoveDownward = false
							}
							if GetPresence(b.Get(rank+i, file)) == Empty || GetColor(b.Get(rank+i, file)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file, Present|b.NextTurn|Rook|HasMoved)
								moves = append(moves, newBoard)
							}
						}
						if canMoveLeft && file-i >= 0 {
							if GetPresence(b.Get(rank, file-i)) == Present {
								canMoveLeft = false
							}
							if GetPresence(b.Get(rank, file-i)) == Empty || GetColor(b.Get(rank, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank, file-i, Present|b.NextTurn|Rook|HasMoved)
								moves = append(moves, newBoard)
							}
						}
						if canMoveRight && file+i <= 7 {
							if GetPresence(b.Get(rank, file+i)) == Present {
								canMoveRight = false
							}
							if GetPresence(b.Get(rank, file+i)) == Empty || GetColor(b.Get(rank, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank, file+i, Present|b.NextTurn|Rook|HasMoved)
								moves = append(moves, newBoard)
							}
						}
					}
				}
			case Knight:
				{
					for _, offset := range []struct {
						i int
						j int
					}{
						{i: -2, j: -1},
						{i: -2, j: 1},
						{i: 2, j: -1},
						{i: 2, j: 1},
						{i: -1, j: -2},
						{i: -1, j: 2},
						{i: 1, j: -2},
						{i: 1, j: 2},
					} {
						if rank+offset.i >= 0 && rank+offset.i <= 7 && file+offset.j >= 0 && file+offset.j <= 7 && (GetPresence(b.Get(rank+offset.i, file+offset.j)) == Empty || GetColor(b.Get(rank+offset.i, file+offset.j)) != b.NextTurn) {
							newBoard := b.DuplicateForNextMove()
							newBoard.Set(rank, file, Empty)
							newBoard.Set(rank+offset.i, file+offset.j, Present|b.NextTurn|Knight)
							moves = append(moves, newBoard)
						}
					}
				}
			case Bishop:
				{
					var canMoveUpRight = rank > 0 && file < 7
					var canMoveDownRight = rank < 7 && file < 7
					var canMoveDownLeft = rank < 7 && file > 0
					var canMoveUpLeft = rank > 0 && file > 0
					for _, i := range moveLengthArray {
						if canMoveUpRight && rank-i >= 0 && file+i <= 7 {
							if GetPresence(b.Get(rank-i, file+i)) == Present {
								canMoveUpRight = false
							}
							if GetPresence(b.Get(rank-i, file+i)) == Empty || GetColor(b.Get(rank-i, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file+i, Present|b.NextTurn|Bishop)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownRight && rank+i <= 7 && file+i <= 7 {
							if GetPresence(b.Get(rank+i, file+i)) == Present {
								canMoveDownRight = false
							}
							if GetPresence(b.Get(rank+i, file+i)) == Empty || GetColor(b.Get(rank+i, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file+i, Present|b.NextTurn|Bishop)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownLeft && rank+i <= 7 && file-i >= 0 {
							if GetPresence(b.Get(rank+i, file-i)) == Present {
								canMoveDownLeft = false
							}
							if GetPresence(b.Get(rank+i, file-i)) == Empty || GetColor(b.Get(rank+i, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file-i, Present|b.NextTurn|Bishop)
								moves = append(moves, newBoard)
							}
						}
						if canMoveUpLeft && rank-i >= 0 && file-i >= 0 {
							if GetPresence(b.Get(rank-i, file-i)) == Present {
								canMoveUpLeft = false
							}
							if GetPresence(b.Get(rank-i, file-i)) == Empty || GetColor(b.Get(rank-i, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file-i, Present|b.NextTurn|Bishop)
								moves = append(moves, newBoard)
							}
						}
					}
				}
			case King:
				{
					for _, offset := range []struct {
						i int
						j int
					}{
						{i: -1, j: 0},
						{i: -1, j: 1},
						{i: 0, j: 1},
						{i: 1, j: 1},
						{i: 1, j: 0},
						{i: 1, j: -1},
						{i: 0, j: -1},
						{i: -1, j: -1},
					} {
						if rank+offset.i >= 0 && rank+offset.i <= 7 && file+offset.j >= 0 && file+offset.j <= 7 && (GetPresence(b.Get(rank+offset.i, file+offset.j)) == Empty || GetColor(b.Get(rank+offset.i, file+offset.j)) != b.NextTurn) {
							newBoard := b.DuplicateForNextMove()
							newBoard.Set(rank, file, Empty)
							newBoard.Set(rank+offset.i, file+offset.j, Present|b.NextTurn|King|HasMoved)
							moves = append(moves, newBoard)
						}
					}
				}
			case Queen:
				{
					// Diagonal moves
					var canMoveUpRight = rank > 0 && file < 7
					var canMoveDownRight = rank < 7 && file < 7
					var canMoveDownLeft = rank < 7 && file > 0
					var canMoveUpLeft = rank > 0 && file > 0

					//Straight moves
					var canMoveUpward = rank > 0
					var canMoveDownward = rank < 7
					var canMoveLeft = file > 0
					var canMoveRight = file < 7

					for _, i := range moveLengthArray {
						if canMoveUpRight && rank-i >= 0 && file+i <= 7 {
							if GetPresence(b.Get(rank-i, file+i)) == Present {
								canMoveUpRight = false
							}
							if GetPresence(b.Get(rank-i, file+i)) == Empty || GetColor(b.Get(rank-i, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file+i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownRight && rank+i <= 7 && file+i <= 7 {
							if GetPresence(b.Get(rank+i, file+i)) == Present {
								canMoveDownRight = false
							}
							if GetPresence(b.Get(rank+i, file+i)) == Empty || GetColor(b.Get(rank+i, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file+i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownLeft && rank+i <= 7 && file-i >= 0 {
							if GetPresence(b.Get(rank+i, file-i)) == Present {
								canMoveDownLeft = false
							}
							if GetPresence(b.Get(rank+i, file-i)) == Empty || GetColor(b.Get(rank+i, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file-i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveUpLeft && rank-i >= 0 && file-i >= 0 {
							if GetPresence(b.Get(rank-i, file-i)) == Present {
								canMoveUpLeft = false
							}
							if GetPresence(b.Get(rank-i, file-i)) == Empty || GetColor(b.Get(rank-i, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file-i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}

						if canMoveUpward && rank-i >= 0 {
							if GetPresence(b.Get(rank-i, file)) == Present {
								canMoveUpward = false
							}
							if GetPresence(b.Get(rank-i, file)) == Empty || GetColor(b.Get(rank-i, file)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank-i, file, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveDownward && rank+i <= 7 {
							if GetPresence(b.Get(rank+i, file)) == Present {
								canMoveDownward = false
							}
							if GetPresence(b.Get(rank+i, file)) == Empty || GetColor(b.Get(rank+i, file)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank+i, file, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveLeft && file-i >= 0 {
							if GetPresence(b.Get(rank, file-i)) == Present {
								canMoveLeft = false
							}
							if GetPresence(b.Get(rank, file-i)) == Empty || GetColor(b.Get(rank, file-i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank, file-i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
						if canMoveRight && file+i <= 7 {
							if GetPresence(b.Get(rank, file+i)) == Present {
								canMoveRight = false
							}
							if GetPresence(b.Get(rank, file+i)) == Empty || GetColor(b.Get(rank, file+i)) != b.NextTurn {
								newBoard := b.DuplicateForNextMove()
								newBoard.Set(rank, file, Empty)
								newBoard.Set(rank, file+i, Present|b.NextTurn|Queen)
								moves = append(moves, newBoard)
							}
						}
					}
				}
			}
		}
	}
	return moves
}

func (b *Board) IsOpponentInCheck() bool {
	opponent := b.NextTurn ^ ColorBits

	// Find king and check if it is in check
	rank := -1
	file := -1
	for searchRank := range []int{0, 1, 2, 3, 4, 5, 6, 7} {
		for searchFile := range []int{0, 1, 2, 3, 4, 5, 6, 7} {
			piece := b.Get(searchRank, searchFile)
			if GetType(piece) == King && GetColor(piece) == opponent {
				rank = searchRank
				file = searchFile
				break
			}
		}
	}
	if rank == -1 || file == -1 {
		return true
	}

	// Diagonal moves
	var canMoveUpRight = rank > 0 && file < 7
	var canMoveDownRight = rank < 7 && file < 7
	var canMoveDownLeft = rank < 7 && file > 0
	var canMoveUpLeft = rank > 0 && file > 0

	//Straight moves
	var canMoveUpward = rank > 0
	var canMoveDownward = rank < 7
	var canMoveLeft = file > 0
	var canMoveRight = file < 7

	for _, i := range moveLengthArray {
		if canMoveUpRight && rank-i >= 0 && file+i <= 7 {
			if GetPresence(b.Get(rank-i, file+i)) == Present {
				canMoveUpRight = false
				if GetColor(b.Get(rank-i, file+i)) == b.NextTurn {
					if GetType(b.Get(rank-i, file+i)) == Bishop || GetType(b.Get(rank-i, file+i)) == Queen {
						return true
						// }
					} else if i == 1 && GetType(b.Get(rank-i, file+i)) == Pawn && b.NextTurn == Black {
						return true
					}
				}
			}
		}
		if canMoveDownRight && rank+i <= 7 && file+i <= 7 {
			if GetPresence(b.Get(rank+i, file+i)) == Present {
				canMoveDownRight = false
				if GetColor(b.Get(rank+i, file+i)) == b.NextTurn {
					if GetType(b.Get(rank+i, file+i)) == Bishop || GetType(b.Get(rank+i, file+i)) == Queen {
						return true
						// }
					} else if i == 1 && GetType(b.Get(rank+i, file+i)) == Pawn && b.NextTurn == White {
						return true
					}
				}
			}
		}
		if canMoveDownLeft && rank+i <= 7 && file-i >= 0 {
			if GetPresence(b.Get(rank+i, file-i)) == Present {
				canMoveDownLeft = false
				if GetColor(b.Get(rank+i, file-i)) == b.NextTurn {
					if GetType(b.Get(rank+i, file-i)) == Bishop || GetType(b.Get(rank+i, file-i)) == Queen {
						return true
						// }
					} else if i == 1 && GetType(b.Get(rank+i, file-i)) == Pawn && b.NextTurn == White {
						return true
					}
				}
			}
		}
		if canMoveUpLeft && rank-i >= 0 && file-i >= 0 {
			if GetPresence(b.Get(rank-i, file-i)) == Present {
				canMoveUpLeft = false
				if GetColor(b.Get(rank-i, file-i)) == b.NextTurn {
					if GetType(b.Get(rank-i, file-i)) == Bishop || GetType(b.Get(rank-i, file-i)) == Queen {
						return true
						// }
					} else if i == 1 && GetType(b.Get(rank-i, file-i)) == Pawn && b.NextTurn == Black {
						return true
					}
				}
			}
		}

		if canMoveUpward && rank-i >= 0 {
			if GetPresence(b.Get(rank-i, file)) == Present {
				canMoveUpward = false
				if GetColor(b.Get(rank-i, file)) == b.NextTurn && (GetType(b.Get(rank-i, file)) == Rook || GetType(b.Get(rank-i, file)) == Queen) {
					return true
				}
			}
		}
		if canMoveDownward && rank+i <= 7 {
			if GetPresence(b.Get(rank+i, file)) == Present {
				canMoveDownward = false
				if GetColor(b.Get(rank+i, file)) == b.NextTurn && (GetType(b.Get(rank+i, file)) == Rook || GetType(b.Get(rank+i, file)) == Queen) {
					return true
				}
			}
		}
		if canMoveLeft && file-i >= 0 {
			if GetPresence(b.Get(rank, file-i)) == Present {
				canMoveLeft = false
				if GetColor(b.Get(rank, file-i)) == b.NextTurn && (GetType(b.Get(rank, file-i)) == Rook || GetType(b.Get(rank, file-i)) == Queen) {
					return true
				}
			}
		}
		if canMoveRight && file+i <= 7 {
			if GetPresence(b.Get(rank, file+i)) == Present {
				canMoveRight = false
				if GetColor(b.Get(rank, file+i)) == b.NextTurn && (GetType(b.Get(rank, file+i)) == Rook || GetType(b.Get(rank, file+i)) == Queen) {
					return true
				}
			}
		}
	}
	for _, offset := range [][]int{
		{-2, -1},
		{-2, 1},
		{2, -1},
		{2, 1},
		{-1, -2},
		{-1, 2},
		{1, -2},
		{1, 2},
	} {
		rankOffset := offset[0]
		fileOffset := offset[1]
		if rank+rankOffset >= 0 &&
			rank+rankOffset <= 7 &&
			file+fileOffset >= 0 &&
			file+fileOffset <= 7 &&
			(GetPresence(b.Get(rank+rankOffset, file+fileOffset)) == Present &&
				GetColor(b.Get(rank+rankOffset, file+fileOffset)) == b.NextTurn &&
				GetType(b.Get(rank+rankOffset, file+fileOffset)) == Knight) {
			return true
		}
	}

	return false
}

func (b *Board) WriteToFile(filename string) {
	file, err := os.Create(filename)
	if err != nil {
		fmt.Printf("Error creating file: %v\n", err)
		os.Exit(1)
	}
	defer file.Close()
	bytes, err := json.Marshal(b)
	if err != nil {
		fmt.Printf("Error marshalling: %v\n", err)
		os.Exit(1)
	}
	file.Write(bytes)
}

func (b *Board) GetEvaluation() *BoardState {
	moves := b.FindMoves()
	legalMoves := []Board{}
	for _, move := range moves {
		if !move.IsOpponentInCheck() {
			legalMoves = append(legalMoves, move)
		}
	}
	if len(legalMoves) == 0 {
		flippedBoard := *b
		flippedBoard.NextTurn = NegateColor(flippedBoard.NextTurn)
		if flippedBoard.IsOpponentInCheck() {
			return &BoardState{
				SelfEvaluation: Evaluation{
					Result: Loss,
					Score:  0,
				},
				Mutex:        nil,
				NextMoves:    []Board{},
				NextBestMove: EmptyBoard,
				AllMoves:     moves,
			}
		} else {
			return &BoardState{
				SelfEvaluation: Evaluation{
					Result: Draw,
					Score:  0,
				},
				Mutex:        nil,
				NextMoves:    []Board{},
				NextBestMove: EmptyBoard,
				AllMoves:     moves,
			}
		}
	}
	material := 0
	for _, piece := range b.Pieces {
		if GetPresence(piece) == Present {
			multiplier := 1
			if GetColor(piece) == b.NextTurn {
				multiplier = 1
			} else {
				multiplier = -1
			}
			material += GetMaterialValue(piece) * multiplier
		}
	}
	return &BoardState{
		SelfEvaluation: Evaluation{
			Result: Scored,
			Score:  material,
		},
		Mutex:        nil,
		NextMoves:    legalMoves,
		NextBestMove: EmptyBoard,
		AllMoves:     moves,
	}
}

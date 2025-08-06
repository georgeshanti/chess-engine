package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/georgeshanti/chess-engine/internal"
)

func main() {
	losingBoard := internal.Board{}
	losingBoardFile, err := os.ReadFile("lost_position.json")
	if err != nil {
		fmt.Printf("Error reading file: %v\n", err)
		os.Exit(1)
	}
	json.Unmarshal(losingBoardFile, &losingBoard)

	winningBoard := internal.Board{}
	winningBoardFile, err := os.ReadFile("winning_position.json")
	if err != nil {
		fmt.Printf("Error reading file: %v\n", err)
		os.Exit(1)
	}
	json.Unmarshal(winningBoardFile, &winningBoard)
	fmt.Printf("Winning board: %v\n", &winningBoard)
	fmt.Printf("Is opponent in check: %t\n", winningBoard.IsOpponentInCheck())

	// eval := losingBoard.GetEvaluation()
	// for _, move := range eval.NextMoves {
	// 	fmt.Printf("Move:\n%v\n", &move)
	// }
}

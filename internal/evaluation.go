package internal

import "sync"

type Result int

var Win Result = 3
var Scored Result = 2
var Draw Result = 1
var Loss Result = 0

type Evaluation struct {
	Result Result
	Score  int
}

func (score Evaluation) CompareTo(other Evaluation) int {
	if score.Result > other.Result {
		return 1
	} else if score.Result < other.Result {
		return -1
	} else if score.Result == Win && score.Score < other.Score {
		return 1
	} else if score.Result == Loss && score.Score > other.Score {
		return 1
	} else {
		return 0
	}
}

func (score Evaluation) Invert() Evaluation {
	switch score.Result {
	case Win:
		return Evaluation{Result: Loss, Score: score.Score + 1}
	case Scored:
		return Evaluation{Result: Scored, Score: 0 - score.Score}
	case Draw:
		return Evaluation{Result: Draw, Score: score.Score + 1}
	case Loss:
		return Evaluation{Result: Win, Score: score.Score + 1}
	}
	panic("Invalid inversion")
}

type BoardState struct {
	SelfEvaluation Evaluation
	Mutex          *sync.Mutex

	PreviousMoves []Board

	NextMoves              []Board
	NextBestMove           Board
	NextBestMoveEvaluation Evaluation

	AllMoves []Board
}

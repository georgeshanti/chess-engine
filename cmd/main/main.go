package main

import (
	"fmt"
	"os"
	"regexp"
	"slices"
	"sync"
	"time"

	"github.com/georgeshanti/chess-engine/internal"
)

var currentPosition internal.Board

var positions = map[internal.Board]*internal.BoardState{}
var positionsMutex = new(sync.Mutex)
var highestDepth int = 0

var pruneMutex = new(sync.RWMutex)

var Duration = time.Now()

func evaluate(board internal.Board, nextMoves []internal.Board, depth int) {
	// fmt.Printf("Sending channel message\n")
	go func() {
		evaluate(board, nextMoves, depth)
	}()
	// fmt.Printf("Sent channel message\n")
}

func printPositions(depth int) {
	positionsMutex.Lock()
	if time.Now().Sub(Duration) > 1000000000 {
		if depth > highestDepth {
			highestDepth = depth
		}
		fmt.Printf("Positions evaluated: %d, Highest depth: %d\n", len(positions), highestDepth)
		Duration = time.Now()
	}
	positionsMutex.Unlock()
}

func _evaluate(board internal.Board, nextMoves []internal.Board, depth int) {
	pruneMutex.RLock()
	defer pruneMutex.RUnlock()
	for _, nextMove := range nextMoves {
		positionsMutex.Lock()
		_, ok := positions[board]
		if !ok {
			positionsMutex.Unlock()
			return
		}
		boardState, ok := positions[nextMove]
		if ok && (boardState == nil || boardState.Mutex == nil) {
			fmt.Printf("Exists: %t, Board state: %v\n", ok, boardState)
			fmt.Printf("Board: %v\n", &nextMove)
		}
		if !ok {
			newBoardState := &internal.BoardState{
				Mutex: new(sync.Mutex),
			}
			newBoardState.Mutex.Lock()
			positions[nextMove] = newBoardState

			positionsMutex.Unlock()

			evaluatedBoardState := nextMove.GetEvaluation()
			newBoardState.SelfEvaluation = evaluatedBoardState.SelfEvaluation
			newBoardState.NextMoves = evaluatedBoardState.NextMoves
			newBoardState.NextBestMove = evaluatedBoardState.NextBestMove
			newBoardState.AllMoves = evaluatedBoardState.AllMoves
			newBoardState.PreviousMoves = append(newBoardState.PreviousMoves, board)
			nextMoves := make([]internal.Board, len(newBoardState.NextMoves))
			copy(nextMoves, newBoardState.NextMoves)
			newBoardState.Mutex.Unlock()
			evaluate(nextMove, nextMoves, depth+1)
			go func() {
				propogate(nextMove)
			}()
		} else {
			boardState.Mutex.Lock()
			positionsMutex.Unlock()
			if !slices.Contains(boardState.PreviousMoves, board) {
				boardState.PreviousMoves = append(boardState.PreviousMoves, board)
			}
			boardState.Mutex.Unlock()
		}
	}
}

func propogate(board internal.Board) {
	pruneMutex.RLock()
	positionsMutex.Lock()
	boardState, _ := positions[board]
	boardState.Mutex.Lock()
	positionsMutex.Unlock()
	for _, previousMove := range boardState.PreviousMoves {
		go func() {
			reevaluate(previousMove)
		}()
	}
	boardState.Mutex.Unlock()
	pruneMutex.RUnlock()
}

func reevaluate(board internal.Board) {
	pruneMutex.RLock()
	positionsMutex.Lock()
	boardState, ok := positions[board]
	if !ok {
		positionsMutex.Unlock()
		pruneMutex.RUnlock()
		return
	}
	boardState.Mutex.Lock()
	nextMoves := make([]internal.Board, len(boardState.NextMoves))
	copy(nextMoves, boardState.NextMoves)

	foundOneMove := false
	nextBestMove := internal.EmptyBoard
	nextBestMoveEvaluation := internal.Evaluation{}
	currentBestMoveEvaluation := boardState.NextBestMoveEvaluation
	for i, nextMove := range nextMoves {
		if i == 0 {
			foundOneMove = true
			nextBestMove = nextMove
			nextBestMoveEvaluation = boardState.NextMoves[i].GetEvaluation().SelfEvaluation
			continue
		}
	}

	if foundOneMove && nextBestMoveEvaluation.CompareTo(currentBestMoveEvaluation) > 0 {
		boardState.NextBestMove = nextBestMove
		boardState.NextBestMoveEvaluation = nextBestMoveEvaluation
		for _, previousMove := range boardState.PreviousMoves {
			go func() {
				reevaluate(previousMove)
			}()
		}
	}

	boardState.Mutex.Unlock()
	positionsMutex.Unlock()
	pruneMutex.RUnlock()
}

var moveRegex = regexp.MustCompile("^[a-h][1-8][a-h][1-8]$")

func prune(move internal.Board) {
	moveBoardState, ok := positions[move]
	if !ok {
		return
	}
	if len(moveBoardState.PreviousMoves) == 0 && currentPosition != move {
		delete(positions, move)
		for _, nextMove := range moveBoardState.NextMoves {
			nextMoveBoardState, ok := positions[nextMove]
			if !ok {
				continue
			}
			index := slices.Index(nextMoveBoardState.PreviousMoves, move)
			if index != -1 {
				nextMoveBoardState.PreviousMoves = slices.Delete(nextMoveBoardState.PreviousMoves, index, index+1)
			}
			if len(nextMoveBoardState.PreviousMoves) == 0 {
				delete(positions, nextMove)
			}
			prune(nextMove)
		}
	}
}

type positionItem struct {
	board internal.Board
	depth int
}

type LinkedListOfListsNode[T any] struct {
	value []T
	next  *LinkedListOfListsNode[T]
}

type LinkedListOfLists[T any] struct {
	head *LinkedListOfListsNode[T]
	tail *LinkedListOfListsNode[T]
}

func (list *LinkedListOfLists[T]) AddList(value []T) {
	if list.head == nil && list.tail == nil {
		// fmt.Printf("Adding list to empty list\n")
		node := &LinkedListOfListsNode[T]{
			value: value,
			next:  nil,
		}
		list.head = node
		list.tail = node
	} else {
		// fmt.Printf("Adding list to non-empty list\n")
		node := &LinkedListOfListsNode[T]{
			value: value,
			next:  nil,
		}
		list.tail.next = node
		list.tail = node
	}
}

func (list *LinkedListOfLists[T]) Dequeue() T {
	if list.head == nil {
		panic("Empty list")
	} else if len(list.head.value) > 0 {
		node := list.head.value[0]
		if len(list.head.value) > 1 {
			list.head.value = list.head.value[1:]
		} else {
			list.head = list.head.next
			if list.head == nil {
				list.tail = nil
			}
		}
		return node
	} else {
		panic("Empty list")
	}
}

func (list *LinkedListOfLists[T]) IsEmpty() bool {
	// fmt.Printf("Checking if list is empty: %t\n", list.head == nil)
	return list.head == nil || len(list.head.value) == 0
}

func main() {
	var positionEvaluations = make(map[internal.Board]internal.BoardState)
	positionsToEvaluate := &LinkedListOfLists[positionItem]{}
	positionsToEvaluate.AddList([]positionItem{{board: internal.InitialBoard, depth: 0}})
	go func() {
		time.Sleep(10 * time.Second)
		fmt.Printf("Exiting\n")
		fmt.Printf("Positions evaluated: %d, highest depth: %d\n", len(positionEvaluations), highestDepth)
		os.Exit(0)
	}()
	for !positionsToEvaluate.IsEmpty() {
		nextPosition := positionsToEvaluate.Dequeue()
		if nextPosition.depth > highestDepth {
			highestDepth = nextPosition.depth
		}
		// fmt.Printf("Positions evaluated: %d, Current depth: %d, highest depth: %d\n", len(positionEvaluations), nextPosition.depth, highestDepth)
		_, ok := positionEvaluations[nextPosition.board]
		if !ok {
			nextPositionEvaluation := *(nextPosition.board.GetEvaluation())
			positionEvaluations[nextPosition.board] = nextPositionEvaluation
			// fmt.Printf("Next position evaluation length: %d\n", len(nextPositionEvaluation.NextMoves))
			positions := make([]positionItem, len(nextPositionEvaluation.NextMoves))
			for i, move := range nextPositionEvaluation.NextMoves {
				positions[i] = positionItem{board: move, depth: nextPosition.depth + 1}
			}
			positionsToEvaluate.AddList(positions)
		}
	}
}

func main1() {
	fmt.Printf("Started evaluation engines\n")
	time.Sleep(1000 * time.Millisecond)
	currentPosition = internal.InitialBoard
	evaluation := currentPosition.GetEvaluation()
	evaluation.Mutex = new(sync.Mutex)
	positions[currentPosition] = evaluation
	evaluate(currentPosition, evaluation.NextMoves, 0)

	for true {
		nextSetOfMoves := currentPosition.GetEvaluation().NextMoves
		fmt.Printf("Current Position: %v\n", &currentPosition)
		fmt.Printf("Enter move: ")
		var move string
		fmt.Scanf("%s", &move)
		if !moveRegex.MatchString(move) {
			fmt.Println("Invalid move. Syntax error.")
			continue
		}
		fromFile := int(move[0] - 'a')
		fromRank := 7 - int(move[1]-'1')
		toFile := int(move[2] - 'a')
		toRank := 7 - int(move[3]-'1')
		currentSourcePiece := currentPosition.Get(fromRank, fromFile)
		currentTargetPiece := currentPosition.Get(toRank, toFile)
		if internal.GetPresence(currentSourcePiece) == internal.Empty {
			fmt.Println("Invalid move. No piece at source position.")
			continue
		}
		fmt.Printf("Current source piece is white: %t\n", internal.GetColor(currentSourcePiece) == internal.White)
		fmt.Printf("Current source piece is black: %t\n", internal.GetColor(currentSourcePiece) == internal.Black)
		if internal.GetColor(currentSourcePiece) != internal.White {
			fmt.Println("Invalid move. Wrong color at source position.")
			continue
		}
		if internal.GetPresence(currentTargetPiece) == internal.Present &&
			internal.GetColor(currentTargetPiece) == internal.White {
			fmt.Println("Invalid move. Own piece at target position.")
			continue
		}
		foundNextMove := false
		nextMove := internal.EmptyBoard
		for _, possibleMove := range nextSetOfMoves {
			nextMoveSourcePiece := possibleMove.Get(fromRank, fromFile)
			nextMoveTargetPiece := possibleMove.Get(toRank, toFile)
			if internal.GetPresence(nextMoveSourcePiece) == internal.Present ||
				internal.GetPresence(nextMoveTargetPiece) == internal.Empty ||
				internal.GetColor(nextMoveTargetPiece) != internal.White {
				continue
			}
			foundNextMove = true
			nextMove = possibleMove
			break
		}
		if !foundNextMove {
			fmt.Println("Invalid move")
			continue
		}

		fmt.Println("Waiting to Prune..")
		// ctx, cancel := context.WithCancel(context.Background())
		// monitor(ctx)
		pruneMutex.Lock()
		// cancel()
		fmt.Println("Pruning started.")
		positionsMutex.Lock()

		previousPosition := currentPosition
		currentPosition = positions[nextMove].NextBestMove
		prune(previousPosition)

		positionsMutex.Unlock()
		fmt.Println("Pruning completed.")
		pruneMutex.Unlock()

		time.Sleep(0)
	}
}

// func monitor(ctx context.Context) {
// 	go func(ctx context.Context) {
// 		for {
// 			select {
// 			case <-ctx.Done(): // if cancel() execute
// 				return
// 			default:
// 				fmt.Printf("Pending readers: %d\n", pruneMutex.ReaderCount())
// 			}

// 			time.Sleep(1000 * time.Millisecond)
// 		}
// 	}(ctx)
// }

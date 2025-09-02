package man

import (
	"fmt"
	"os"
	"regexp"
	"runtime"
	"slices"
	"sync"
	"time"

	"github.com/georgeshanti/chess-engine/internal"
)

var currentPosition internal.Board

var positionsMutex = new(sync.RWMutex)
var positions = map[internal.Board]*internal.BoardState{}

var highestDepth int = 0

var pruneMutex = new(sync.RWMutex)
var wg = new(sync.WaitGroup)

var moveRegex = regexp.MustCompile("^[a-h][1-8][a-h][1-8]$")

type positionItem struct {
	parent internal.Board
	board  internal.Board
	depth  int
}

var positionsToEvaluate = NewLinkedListOfLists[positionItem]()
var mutex = new(sync.Mutex)
var duplications = 0

var iterations = make(map[int]*struct {
	value int
	lock  *sync.Mutex
})
var start = time.Now()

func evaluationEngine(i int) {
	fmt.Printf("Evaluation engine %d started\n", i)
	var index = 0
	for time.Since(start) < 10*time.Second {
		nextPosition := positionsToEvaluate.Dequeue()
		iterations[i].lock.Lock()
		iterations[i].value++
		iterations[i].lock.Unlock()
		evaluate(nextPosition, false, fmt.Sprintf("%d:%d", i, index))
		index++
	}
	fmt.Printf("Evaluation engine %d finished\n", i)
	wg.Add(-1)
}

func evaluate(nextPosition positionItem, spawnRoutines bool, index string) {
	positionsMutex.RLock()
	boardState, ok := positions[nextPosition.board]
	if nextPosition.depth > highestDepth {
		highestDepth = nextPosition.depth
	}
	if ok {
		boardState.Mutex.Lock()
		positionsMutex.RUnlock()

		if !slices.Contains(boardState.PreviousMoves, nextPosition.parent) {
			boardState.PreviousMoves = append(boardState.PreviousMoves, nextPosition.parent)
		}
		boardState.Mutex.Unlock()
		mutex.Lock()
		duplications++
		mutex.Unlock()
	} else {
		positionsMutex.RUnlock()
		positionsMutex.Lock()
		newBoardState := &internal.BoardState{
			Mutex: new(sync.Mutex),
		}
		positions[nextPosition.board] = newBoardState
		positionsMutex.Unlock()

		// fmt.Printf("Started evaluation of position %s\n", index)
		evaluatedBoardState := nextPosition.board.GetEvaluation()
		// fmt.Printf("Stopped evaluation of position %s\n", index)
		newBoardState.SelfEvaluation = evaluatedBoardState.SelfEvaluation
		newBoardState.NextMoves = evaluatedBoardState.NextMoves
		newBoardState.NextBestMove = evaluatedBoardState.NextBestMove
		newBoardState.AllMoves = evaluatedBoardState.AllMoves

		newBoardState.Mutex.Lock()
		newBoardState.PreviousMoves = append(newBoardState.PreviousMoves, nextPosition.parent)
		newBoardState.Mutex.Unlock()

		if !spawnRoutines {
			nextPositions := make([]positionItem, len(newBoardState.NextMoves))
			for i, move := range evaluatedBoardState.NextMoves {
				nextPositions[i] = positionItem{parent: nextPosition.board, board: move, depth: nextPosition.depth + 1}
			}
			if len(nextPositions) > 0 {
				positionsToEvaluate.AddList(nextPositions)
			} else {
				fmt.Printf("No next positions: %d\n", len(positions))
			}
		} else {
			for _, move := range evaluatedBoardState.NextMoves {
				go evaluate(positionItem{parent: nextPosition.board, board: move, depth: nextPosition.depth + 1}, true, "")
			}
		}
	}
}

func runWithGoroutines() {
	go func() {
		time.Sleep(10 * time.Second)
		fmt.Printf("Exiting\n")
		fmt.Printf("Positions evaluated: %d, highest depth: %d, iterations: %d\n", len(positions), highestDepth, iterations)
		fmt.Printf("Goroutines: %d\n", runtime.NumGoroutine())
		os.Exit(0)
	}()
	go evaluate(positionItem{board: internal.InitialBoard, depth: 0}, true, "")
	channel := make(chan int)
	<-channel
}

func runWithEvaluationEngines() {
	positionsToEvaluate.AddList([]positionItem{{board: internal.InitialBoard, depth: 0}})
	// go func() {
	// 	time.Sleep(10 * time.Second)
	// 	fmt.Printf("Exiting\n")
	// 	fmt.Printf("Positions evaluated: %d, highest depth: %d, iterations: %d\n", len(positions), highestDepth, iterations)
	// 	fmt.Printf("Goroutines: %d\n", runtime.NumGoroutine())
	// 	os.Exit(0)
	// }()
	cpuCount := runtime.NumCPU() - 1
	cpuCount = 1
	for i := 0; i < cpuCount; i++ {
		fmt.Printf("Starting evaluation engine %d\n", i)
		wg.Add(1)
		iterations[i] = &struct {
			value int
			lock  *sync.Mutex
		}{
			value: 0,
			lock:  new(sync.Mutex),
		}
		go evaluationEngine(i)
	}
	wg.Wait()
	fmt.Printf("Evaluation engines finished\n")
	fmt.Printf("Exiting\n")
	iterationsMap := make(map[int]int)
	for k, v := range iterations {
		iterationsMap[k] = v.value
	}
	fmt.Printf("Positions evaluated: %d, highest depth: %d, iterations: %v\n", len(positions), highestDepth, iterationsMap)
	fmt.Printf("Duplications: %d\n", duplications)
	fmt.Printf("Goroutines: %d\n", runtime.NumGoroutine())

	// evaluationEngine(0)
}

func Main() {
	// runWithGoroutines()
	runWithEvaluationEngines()
}

// func main1() {
// 	fmt.Printf("Started evaluation engines\n")
// 	time.Sleep(1000 * time.Millisecond)
// 	currentPosition = internal.InitialBoard
// 	evaluation := currentPosition.GetEvaluation()
// 	evaluation.Mutex = new(sync.Mutex)
// 	positions[currentPosition] = evaluation
// 	// evaluate(currentPosition, evaluation.NextMoves, 0)

// 	for true {
// 		nextSetOfMoves := currentPosition.GetEvaluation().NextMoves
// 		fmt.Printf("Current Position: %v\n", &currentPosition)
// 		fmt.Printf("Enter move: ")
// 		var move string
// 		fmt.Scanf("%s", &move)
// 		if !moveRegex.MatchString(move) {
// 			fmt.Println("Invalid move. Syntax error.")
// 			continue
// 		}
// 		fromFile := int(move[0] - 'a')
// 		fromRank := 7 - int(move[1]-'1')
// 		toFile := int(move[2] - 'a')
// 		toRank := 7 - int(move[3]-'1')
// 		currentSourcePiece := currentPosition.Get(fromRank, fromFile)
// 		currentTargetPiece := currentPosition.Get(toRank, toFile)
// 		if internal.GetPresence(currentSourcePiece) == internal.Empty {
// 			fmt.Println("Invalid move. No piece at source position.")
// 			continue
// 		}
// 		fmt.Printf("Current source piece is white: %t\n", internal.GetColor(currentSourcePiece) == internal.White)
// 		fmt.Printf("Current source piece is black: %t\n", internal.GetColor(currentSourcePiece) == internal.Black)
// 		if internal.GetColor(currentSourcePiece) != internal.White {
// 			fmt.Println("Invalid move. Wrong color at source position.")
// 			continue
// 		}
// 		if internal.GetPresence(currentTargetPiece) == internal.Present &&
// 			internal.GetColor(currentTargetPiece) == internal.White {
// 			fmt.Println("Invalid move. Own piece at target position.")
// 			continue
// 		}
// 		foundNextMove := false
// 		nextMove := internal.EmptyBoard
// 		for _, possibleMove := range nextSetOfMoves {
// 			nextMoveSourcePiece := possibleMove.Get(fromRank, fromFile)
// 			nextMoveTargetPiece := possibleMove.Get(toRank, toFile)
// 			if internal.GetPresence(nextMoveSourcePiece) == internal.Present ||
// 				internal.GetPresence(nextMoveTargetPiece) == internal.Empty ||
// 				internal.GetColor(nextMoveTargetPiece) != internal.White {
// 				continue
// 			}
// 			foundNextMove = true
// 			nextMove = possibleMove
// 			break
// 		}
// 		if !foundNextMove {
// 			fmt.Println("Invalid move")
// 			continue
// 		}

// 		fmt.Println("Waiting to Prune..")
// 		// ctx, cancel := context.WithCancel(context.Background())
// 		// monitor(ctx)
// 		pruneMutex.Lock()
// 		// cancel()
// 		fmt.Println("Pruning started.")
// 		positionsMutex.Lock()

// 		previousPosition := currentPosition
// 		currentPosition = positions[nextMove].NextBestMove
// 		prune(previousPosition)

// 		positionsMutex.Unlock()
// 		fmt.Println("Pruning completed.")
// 		pruneMutex.Unlock()

// 		time.Sleep(0)
// 	}
// }

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

package main

import (
	"fmt"
	"os"
	"runtime"
	"runtime/pprof"

	"github.com/georgeshanti/chess-engine/internal/man"
)

func main() {
	runtime.SetMutexProfileFraction(1)
	TimerWrapper(man.Main)()
}

type VoidFunc func()

func TimerWrapper(fn VoidFunc) func() {
	return func() {
		fmt.Println("Profiling started")
		cpuprofile1 := "./pprof/cpu.pprof"
		f, err := os.Create(cpuprofile1)
		if err != nil {
			fmt.Fprintf(os.Stderr, "could not create CPU profile: %v\n", err)
			panic(err)
		}
		if err := pprof.StartCPUProfile(f); err != nil {
			fmt.Fprintf(os.Stderr, "could not start CPU profile: %v\n", err)
			panic(err)
		}

		fn()

		pprof.StopCPUProfile()
		fmt.Println("  ...Complete")
		fmt.Println()
	}
}

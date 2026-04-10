package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gorilla/websocket"
)

func main() {
	var upgrader = websocket.Upgrader{
		ReadBufferSize:  1024,
		WriteBufferSize: 1024,
	}

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		// Upgrade upgrades the HTTP server connection to the WebSocket protocol.
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Print("upgrade failed: ", err)
			return
		}
		defer conn.Close()

		// Continuosly read and write message
		f, err := os.Open("..//logs/mem/log.txt")
		if err != nil {
			panic(err)
		}
		defer f.Close()
		buf := make([]byte, 77)
		fmt.Println("Starting")
		for {
			n, err := f.Read(buf)
			if err == io.EOF {
				// there is no more data to read
				break
			}
			if err != nil {
				fmt.Println(err)
				continue
			}
			if n > 0 {
				err := conn.WriteMessage(websocket.TextMessage, buf)
				if err != nil {
					break
				}
				time.Sleep(100 * time.Millisecond)
			}
		}
		fmt.Println("Done")
	})

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.ServeFile(w, r, "wes.html")
	})

	http.ListenAndServe(":8080", nil)
}

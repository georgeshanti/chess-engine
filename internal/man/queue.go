package man

import (
	"sync"
)

type LinkedListOfListsNode[T any] struct {
	value []T
	next  *LinkedListOfListsNode[T]
}

type LinkedListOfLists[T any] struct {
	value []T

	headLock *sync.Mutex
	head     *LinkedListOfListsNode[T]

	tailLock *sync.Mutex
	tail     *LinkedListOfListsNode[T]

	dequeueWaitersLock *sync.Mutex
	dequeueWaiters     []chan int
}

func NewLinkedListOfLists[T any]() *LinkedListOfLists[T] {
	return &LinkedListOfLists[T]{
		value:              []T{},
		headLock:           &sync.Mutex{},
		tailLock:           &sync.Mutex{},
		dequeueWaitersLock: &sync.Mutex{},
		dequeueWaiters:     []chan int{},
	}
}

func (list *LinkedListOfLists[T]) AddList(value []T) {
	// list.headLock.Lock()
	// list.value = append(list.value, value...)
	// list.headLock.Unlock()
	// return

	if len(value) == 0 {
		return
	}

	list.tailLock.Lock()
	if list.tail == nil {
		list.headLock.Lock()
		node := &LinkedListOfListsNode[T]{
			value: value,
			next:  nil,
		}
		list.head = node
		list.tail = node
		list.dequeueWaitersLock.Lock()
		for _, channel := range list.dequeueWaiters {
			channel <- 1
		}
		list.dequeueWaiters = []chan int{}
		list.dequeueWaitersLock.Unlock()
		list.headLock.Unlock()
	} else {
		// fmt.Printf("Adding list to non-empty list\n")
		node := &LinkedListOfListsNode[T]{
			value: value,
			next:  nil,
		}
		list.tail.next = node
		list.tail = node
	}
	list.tailLock.Unlock()
}

func slice[T any](list []T) []T {
	return list[1:]
}

func (list *LinkedListOfLists[T]) Dequeue() T {

	// list.headLock.Lock()
	// var node T
	// if len(list.value) > 0 {
	// 	node = list.value[0]
	// 	list.value = list.value[1:]
	// }
	// list.headLock.Unlock()
	// return node

	list.headLock.Lock()
	for list.head == nil {
		list.headLock.Unlock()
		list.dequeueWaitersLock.Lock()
		channel := make(chan int)
		list.dequeueWaiters = append(list.dequeueWaiters, channel)
		list.dequeueWaitersLock.Unlock()
		<-channel
		list.headLock.Lock()
	}
	if len(list.head.value) > 0 {
		node := list.head.value[0]
		if len(list.head.value) > 1 {
			list.head.value = slice(list.head.value)
		} else {
			list.tailLock.Lock()
			list.head = list.head.next
			if list.head == nil {
				list.tail = nil
			}
			list.tailLock.Unlock()
		}
		list.headLock.Unlock()
		return node
	} else {
		panic("Empty list")
	}
}

func (list *LinkedListOfLists[T]) IsEmpty() bool {
	// fmt.Printf("Checking if list is empty: %t\n", list.head == nil)
	return list.head == nil || len(list.head.value) == 0
}

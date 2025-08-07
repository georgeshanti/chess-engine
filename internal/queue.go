package internal

import (
	"sync"
)

type LinkedListOfListsNode[T any] struct {
	value []T
	next  *LinkedListOfListsNode[T]
}

type LinkedListOfLists[T any] struct {
	headLock *sync.Mutex
	head     *LinkedListOfListsNode[T]

	tailLock *sync.Mutex
	tail     *LinkedListOfListsNode[T]

	dequeueWaitersLock *sync.Mutex
	dequeueWaiters     []chan int
}

func NewLinkedListOfLists[T any]() *LinkedListOfLists[T] {
	return &LinkedListOfLists[T]{
		headLock:           &sync.Mutex{},
		tailLock:           &sync.Mutex{},
		dequeueWaitersLock: &sync.Mutex{},
		dequeueWaiters:     []chan int{},
	}
}

func (list *LinkedListOfLists[T]) AddList(value []T) {
	list.tailLock.Lock()
	defer list.tailLock.Unlock()
	if list.tail == nil {
		list.headLock.Lock()
		defer list.headLock.Unlock()
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
	list.headLock.Lock()
	defer list.headLock.Unlock()
	for list.head == nil {
		list.headLock.Unlock()
		list.dequeueWaitersLock.Lock()
		channel := make(chan int)
		list.dequeueWaiters = append(list.dequeueWaiters, channel)
		list.dequeueWaitersLock.Unlock()
		<-list.dequeueWaiters[len(list.dequeueWaiters)-1]
		list.headLock.Lock()
	}
	if len(list.head.value) > 0 {
		node := list.head.value[0]
		if len(list.head.value) > 1 {
			list.head.value = list.head.value[1:]
		} else {
			list.tailLock.Lock()
			defer list.tailLock.Unlock()
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

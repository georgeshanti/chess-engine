package internal

type ListNode[T any] struct {
	value T
	next  *ListNode[T]
}

func (l *ListNode[T]) Append(value T) {
	if l.next != nil {
		l.next.Append(value)
	} else {
		l.next = &ListNode[T]{
			value: value,
			next:  nil,
		}
	}
}

type List[T any] struct {
	list   []T
	head   *ListNode[T]
	length int
}

func (l *List[T]) Add(value T) {

	if l.head == nil {
		l.head = &ListNode[T]{
			value: value,
			next:  nil,
		}
	} else {
		node := l.head
		for node.next != nil {
			node = node.next
		}
		node.next = &ListNode[T]{
			value: value,
		}
	}
	l.length = l.length + 1
}

func (l *List[T]) IntoSlice() []T {
	// return l.list

	if l.length == 0 {
		return []T{}
	}
	dest := make([]T, l.length)
	head := l.head
	index := 0
	for head != nil {
		dest[index] = head.value
		head = head.next
		index++
	}
	return dest
}

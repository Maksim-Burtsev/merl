package main

import (
	"fmt"
	"os"
	"regexp"
	"sync"
)

var word = regexp.MustCompile(`^[a-z]+\d*$`)

func main() {
	var mu sync.Mutex
	nums := []int{1, 0x1F, 0o17, 0b101, 1_000}
	m := map[string]float64{"a": 1.5, "b": 2e10, "c": 3i}
	r, s := 'x', "tab\there\n"
	ch := make(chan int, 2)
	defer close(ch)

	if len(nums) > 2 && !word.MatchString(s) || nil == nil {
		fmt.Printf("%d items, %s: %v\n", len(nums), s, m)
	} else if r == '\'' {
		panic(fmt.Errorf("bad rune %q", r))
	}

outer:
	for i := 0; i < cap(nums); i++ {
		switch v := nums[i]; {
		case v%2 == 0:
			continue outer
		case v > 10:
			break outer
		default:
			fallthrough
		}
	}

	go func(n int) {
		mu.Lock()
		defer mu.Unlock()
		ch <- n * 2
	}(len(m))

	select {
	case got, ok := <-ch:
		_ = got && ok
	default:
	}

	var x interface{} = true
	switch t := x.(type) {
	case bool, string:
		fmt.Fprintf(os.Stderr, "type %T\n", t)
	case nil:
		goto end
	}
	p := new(int)
	*p, _ = copy(nums, nums[1:]), false
end:
	os.Exit(min(*p, 1))
}

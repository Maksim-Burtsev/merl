// Every construct `f` folds in Go, and the cases that once broke it.
package main

import (  // f: 4-7
	"fmt"  // f: 4-7
	"strings"  // f: 4-7
)  // f: 4-7

const (  // f: 9-12
	small = 1  // f: 9-12
	large = 2  // f: 9-12
)  // f: 9-12

type Point struct {  // f: 14-16
	X, Y int  // f: 14-16
}  // f: 14-16

type Shape interface {  // f: 18-20
	Area() float64  // f: 18-20
}  // f: 18-20

func (p Point) String() string {  // f: 22-24
	return fmt.Sprintf("(%d, %d)", p.X, p.Y)  // f: 22-24
}  // f: 22-24

func classify(n int,  // f: 26-44
	label string,  // f: 26-44
) string {  // f: 28-44
	switch {  // f: 29-43
	case n < small:  // f: 30-32
		return "none"  // f: 26-44

	case n < large:  // f: 33-40
		if label == "" {  // f: 34-40
			return "one"  // f: 26-44
		} else if strings.HasPrefix(label, "x") {  // f: 36-37
			return "x"  // f: 26-44
		} else {  // f: 38-40
			return label  // f: 26-44
		}  // f: 26-44
	default:  // f: 41-42
		return "many"  // f: 26-44
	}  // f: 26-44
}  // f: 26-44

func query() string {  // f: 46-77
	sql := `  // f: 46-77
select *  // f: 46-77
from t  // f: 46-77
`  // f: 46-77
// a comment at column 0 inside the body  // f: 46-77
	rows := []struct {  // f: 52-64
		name string  // f: 46-77
		size int  // f: 46-77
	}{  // f: 46-77
		{  // f: 56-58
			name: "a",  // f: 46-77
			size: 1,  // f: 46-77
		}, {  // f: 59-63
			name: `b  // f: 60-61
c`,  // f: 46-77
			size: 2,  // f: 46-77
		},  // f: 46-77
	}  // f: 46-77
	for _, r := range rows {  // f: 65-69
		go func() {  // f: 66-68
			fmt.Println(r.name)  // f: 66-68
		}()  // f: 66-68
	}  // f: 46-77
	done := make(chan bool)  // f: 46-77
	select {  // f: 46-77
	case <-done:  // f: 46-77
		return sql  // f: 46-77
	default:  // f: 46-77
	}  // f: 46-77
	return sql  // f: 46-77
}  // f: 46-77

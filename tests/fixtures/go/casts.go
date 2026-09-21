package main

import "fmt"

type Shape interface {
	Area() int
}

type Square struct{}

func (Square) Area() int { return 1 }

type Circle struct{}

func (Circle) Area() int { return 3 }

func CastFirst(found interface{}, id int) {
	repo, ok := found.(*UserRepository)
	if ok {
		repo.DeleteUser(id)
	}
	audit := found.(AuditLog)
	audit.DeleteUser(id + 1)
	found.(*UserRepository).DeleteUser(id + 2)
	fmt.Println(found.(fmt.Stringer).String())
}

func CastShapes(shape Shape, found interface{}, id int) int {
	switch v := found.(type) {
	case *UserRepository:
		v.DeleteUser(id + 3)
	case AuditLog:
		if id > 0 {
			v.DeleteUser(id + 4)
		}
	}
	switch v := shape.(type) {
	case Square:
		return v.Area()
	case Circle, *Circle:
		return v.Area() + 1
	default:
		return v.Area() + 2
	}
}

package main

import (
	"fmt"

	"example.com/fixture/go-ledger"
	depot "example.com/fixture/store"
)

// A constant's value spells names and declares none of them.
const qualifierNote = "fmt and depot are imports"

// `d` on a package qualifier is its import line (#100); on a local of the name, the local.
func QualifierOpen() {
	depot.Open().Close()
	fmt.Println("open")
}

func QualifierHidden(depot *UserService) {
	depot.Remove(1)
}

// The import's path reads as `ledger`, its package is `books`, and `ledger` is the package's
// own variable of `scopes.go`: no import line is its declaration.
func QualifierGuess() {
	books.Post()
	ledger.DeleteUser(2)
}

// A parameter the scope walk misses, behind a function-typed one, is no import either.
func QualifierFuncParam(depot *UserService, each func()) {
	depot.Remove(2)
}

// A field called like the import is a field.
type qualifierHolder struct {
	depot *UserService
}

func (h qualifierHolder) Run() {
	h.depot.Remove(3)
}

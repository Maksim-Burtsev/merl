package main

import (
	"fmt"

	"example.com/fixture/go-ledger"
	depot "example.com/fixture/store"
)

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

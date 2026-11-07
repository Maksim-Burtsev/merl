package main

import (
	"example.com/fixture/go-ledger"
)

// The path of `go-ledger` reads as `ledger` and its package clause says `books`: `books.Folio` is
// that package's, not the field `Folio` a struct embeds, and `books.Balance` not the fakes' one.
type folioShelf struct {
	books.Folio
}

func Folios() int {
	f := books.Folio{Spine: 1}
	//         ^ d: go-ledger/books.go:6
	return f.Spine + books.Balance()
	//                     ^ d: go-ledger/books.go:10
}

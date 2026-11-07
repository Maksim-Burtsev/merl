package rival

// An import path inside a string imports nothing: `books` here is the variable of folio_var.go.
const ledgerPath = `
"example.com/fixture/go-ledger"
`

func Folio() int {
	return books.Folio + len(ledgerPath)
	//           ^ d: rival/folio_var.go:4
}

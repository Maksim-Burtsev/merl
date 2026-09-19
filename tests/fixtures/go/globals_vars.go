package main

var defaultRepo *UserRepository

var (
	sharedAudit AuditLog
	sharedRepo  = NewRepo()
)

// The second name of a list has a value the rules do not pair up.
var spareRepo, spareAudit = NewRepo(), AuditLog{}

// A `var` inside a function, and a line of a raw string, declare nothing for the package.
func globalsInner() string {
	var sharedAudit *UserRepository
	_ = sharedAudit
	return `
var defaultRepo AuditLog
`
}

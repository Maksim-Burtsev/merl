//go:build js

package main

// Under another build tag the name is another type: the two files disagree.
var taggedRepo AuditLog

// Declared under both tags as one type, and under one of them with no type the rules read.
var twinRepo *UserRepository

var _, mixedRepo = pair()

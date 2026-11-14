//go:build js

package main

// Under another build tag the name is another type: the two files disagree.
var taggedRepo AuditLog

// Declared under both tags as one type, under one of them by the second result of a call.
var twinRepo *UserRepository

var _, mixedRepo = pair()

package main_test

import (
	"testing"

	defaultRepo "example.com/fixture/store"
)

// From the external test package an import spells the name of a variable of `package main`.
func TestGlobals(t *testing.T) {
	session := defaultRepo.Open()
	session.Close()
}

package main

import "example.com/fixture/store"

func Nightly() {
	store.Open().Close()
}

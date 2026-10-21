package infuse

import "github.com/tea/kettle"

func Vendored() int {
	var k kettle.Kettle
	n := k.Rumble()
	//     ^ d: vendor/github.com/tea/kettle/kettle.go:5
	made := kettle.Kettle{}
	return n + made.Rumble()
	//              ^ d: vendor/github.com/tea/kettle/kettle.go:5
}

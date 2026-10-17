package rival

import (
	steep "example.com/fixture/rival/twin"
)

func Chime() int {
	k := steep.Kettle{}
	//         ^ d: rival/twin/twin.go:3
	pour := steep.Kettle.Pour
	//                   ^ d: rival/twin/twin.go:7
	return pour(k) + steep.Whistle()
	//                     ^ d: rival/twin/twin.go:9
}

func Ding(steep Kettle) int {
	return steep.Pour()
	//           ^ d: rival/rival.go:25
}

func Clink() int {
	var k Kettle
	k, ok := Kettle{}, true
	_ = ok
	return k.Pour()
	//       ^ d: rival/rival.go:25
}

package main

import brew "example.com/fixture/infuse"

type Tumbler = brew.Mug

func TumblerSip() int {
	return Tumbler{}.Sip() + brew.Mug{}.Sip() + Tumbler{Handle: 1}.Handle
	//     ^ d: tumblers.go:5
	//               ^ d: infuse/cup.go:33
	//                                  ^ d: infuse/cup.go:33
	//                                                             ^ d: infuse/cup.go:30
}

package main

import "example.com/fixture/platform"

func PlatformTick(clock *platform.Clock, codec *platform.Codec) int {
	made := platform.NewClock()
	return clock.Now() + made.Now() + codec.Encode()
}

package main

import "example.com/fixture/platform"

func PlatformTick(clock *platform.Clock, codec *platform.Codec, timer *platform.Timer, gauge *platform.Gauge) int {
	made := platform.NewClock()
	return clock.Now() + made.Now() + codec.Encode() + timer.Tick() + gauge.Read()
}

//go:build gated

package main

import "example.com/fixture/platform"

func PlatformGate(gate *platform.Gate, meter *platform.Meter) int {
	return gate.Lift() + platform.NewGate().Lift() + meter.Sample()
}

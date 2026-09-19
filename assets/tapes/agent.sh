#!/bin/sh
# Plays the agent in assets/day/6-live.tape: a few seconds after merl opens, it writes a new test
# file into the checkout, the way an agent in the next split would.
sleep "${1:-6}"
cat > services/issue/cap_test.go <<'GO'
package issue

import "testing"

func TestAssigneeCap(t *testing.T) {
	t.Skip("todo")
}
GO

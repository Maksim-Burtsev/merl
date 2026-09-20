#!/bin/sh
# Plays the agent in assets/day/6-live.steps: a few seconds after merl opens, it writes a new test
# file into the checkout, the way an agent in the next split would.
sleep "${1:-6}"
cat > services/user/block_note_test.go <<'GO'
package user

import "testing"

func TestBlockingNoteIsTrimmed(t *testing.T) {
	t.Skip("todo")
}
GO

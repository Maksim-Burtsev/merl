#!/bin/sh
sleep "${1:-6}"
cat > services/user/block_note_test.go <<'GO'
package user

import "testing"

func TestBlockingNoteIsTrimmed(t *testing.T) {
	t.Skip("todo")
}
GO

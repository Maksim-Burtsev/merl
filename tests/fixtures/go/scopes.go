package main

var ledger = &AuditLog{}

func pair() (int, *UserRepository) {
	return 0, NewRepo()
}

func ScopedRotate(id int) {
	ledger := NewRepo()
	ledger.FindUser(id)
}

func ScopedNested(id int, ok bool) {
	ledger := AuditLog{}
	if ok {
		ledger := NewRepo()
		ledger.DeleteUser(id + 1)
	}
	ledger.DeleteUser(id + 2)
}

func ScopedPackage(id int) {
	ledger.DeleteUser(id + 3)
}

func ScopedHidden(id int) {
	_, ledger := pair()
	ledger.DeleteUser(id + 4)
}

func ScopedTwice(id int, ok bool) {
	if ledger := NewRepo(); ok && ledger != nil {
		ledger := AuditLog{}
		ledger.DeleteUser(id + 5)
	}
}

type scopedOptions struct{ run func() }

func scopedRegister(each func(ledger *UserRepository), options scopedOptions) {}

func ScopedLiteral(id int) {
	scopedRegister(func(ledger *UserRepository) {}, scopedOptions{
		run: func() { ledger.DeleteUser(id + 6) },
	})
}

// What a sibling block or a callback on the header's line declares is not the cursor's.
func ScopedSibling(ledger AuditLog, repos []*UserRepository, ok bool) {
	if ok {
		scopedRegister(func(ledger *UserRepository) {
		}, scopedOptions{})
	} else {
		ledger.DeleteUser(7)
	}
	if ok {
		for _, ledger := range repos {
			_ = ledger
		}
	} else {
		ledger.DeleteUser(8)
	}
	if scopedAny(func(ledger *UserRepository) bool { return ledger != nil }) {
		ledger.DeleteUser(9)
	}
}

func scopedAny(each func(ledger *UserRepository) bool) bool { return each(nil) }

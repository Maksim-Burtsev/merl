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

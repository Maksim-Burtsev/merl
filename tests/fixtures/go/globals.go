package main

// Package-level names declared in another file of the package, `globals_vars.go`, and one
// declared below its use in this file (#100).
func GlobalRotate(id int) {
	defaultRepo.DeleteUser(id + 10)
	sharedAudit.DeleteUser(id + 11)
	sharedRepo.DeleteUser(id + 12)
	lateRepo.DeleteUser(id + 13)
	spareAudit.DeleteUser(id + 14)
	taggedRepo.DeleteUser(id + 17)
	twinRepo.DeleteUser(id + 23)
	mixedRepo.DeleteUser(id + 24)
}

// A local of the name hides the package's, whatever the rules read of it.
func GlobalHidden(id int) {
	defaultRepo := AuditLog{}
	defaultRepo.DeleteUser(id + 15)
	_, sharedAudit := pair()
	sharedAudit.DeleteUser(id + 16)
}

var lateRepo = &UserRepository{}

// Locals the scope walk does not read hide the package's name all the same: a header with a
// function-typed parameter, a receiver on one, a `var (` block in a function, the lines above a
// label, a local handed on.
func GlobalFuncParam(defaultRepo AuditLog, each func(id int) error) {
	defaultRepo.DeleteUser(18)
}

func (defaultRepo AuditLog) GlobalReceiver(each func(id int) bool) {
	defaultRepo.DeleteUser(19)
}

func GlobalVarBlock() {
	var (
		defaultRepo = AuditLog{}
	)
	defaultRepo.DeleteUser(20)
}

func GlobalLabel(n int) {
	defaultRepo := AuditLog{}
retry:
	defaultRepo.DeleteUser(21)
	if n > 0 {
		n--
		goto retry
	}
}

func GlobalHop(defaultRepo AuditLog, each func()) {
	hop := defaultRepo
	hop.DeleteUser(22)
}

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
}

// A local of the name hides the package's, whatever the rules read of it.
func GlobalHidden(id int) {
	defaultRepo := AuditLog{}
	defaultRepo.DeleteUser(id + 15)
	_, sharedAudit := pair()
	sharedAudit.DeleteUser(id + 16)
}

var lateRepo = &UserRepository{}

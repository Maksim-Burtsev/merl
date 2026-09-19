package main

import "example.com/fixture/store"

func NewRepo() *UserRepository {
	return &UserRepository{}
}

func NewAudit() (AuditLog, error) {
	return AuditLog{}, nil
}

func Rotate(id int) {
	repo := NewRepo()
	repo.DeleteUser(id)
	audit, err := NewAudit()
	if err == nil {
		audit.DeleteUser(id)
	}
	session := store.Open()
	session.Close()
}

func Forget(repos chan *UserRepository, id int) {
	for repo := range repos {
		repo.FindUser(id)
		repo.DeleteUser(id)
	}
}

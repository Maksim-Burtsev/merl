package main

import "fmt"

type User struct {
	ID int
}

type UserRepository struct{}

func (r *UserRepository) FindUser(id int) User {
	return User{ID: id}
}

func (r *UserRepository) DeleteUser(id int) {
	fmt.Println(id)
}

type AuditLog struct{}

func (a AuditLog) DeleteUser(id int) {
	fmt.Println(id)
}

type Notifier interface {
	Send(text string)
}

type EmailNotifier struct{}

func (e *EmailNotifier) Send(text string) {
	fmt.Println(text)
}

type SmsNotifier struct{}

func (s SmsNotifier) Send(text string) {
	fmt.Println(text)
}

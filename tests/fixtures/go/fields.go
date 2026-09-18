package main

import (
	"fmt"
	"net/http"
)

type Base struct {
	Audit AuditLog
}

type Issue struct {
	Base
	PosterID    int
	Title, Body string
	repo        *UserRepository
}

type Comment struct {
	PosterID int
	Text     string
	Issue    *Issue
}

func (i *Issue) Close() {
	i.repo.DeleteUser(i.PosterID)
}

func Show(issue *Issue, comments []Comment) {
	fmt.Println(issue.PosterID, issue.Body, issue.Audit, issue.Base)
	for _, c := range comments {
		fmt.Println(c.PosterID, c.Text)
	}
}

func Serve(r *http.Request) string {
	var (
		Host string
	)
	Host = r.Host
	return Host
}

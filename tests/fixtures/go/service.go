package main

import "fmt"

type UserService struct {
	repo     *UserRepository
	audit    AuditLog
	notifier Notifier
}

func (s *UserService) Remove(id int) {
	user := s.repo.FindUser(id)
	s.repo.DeleteUser(id)
	s.audit.DeleteUser(id)
	s.notifier.Send(fmt.Sprintf("removed %d", user.ID))
}

type Services struct {
	Users *UserService
}

type App struct {
	Services *Services
}

func HandleDelete(app *App, id int) {
	app.Services.Users.Remove(id)
}

func Cleanup(id int) {
	repo := AuditLog{}
	purge := func() {
		repo := &UserRepository{}
		repo.DeleteUser(id)
	}
	purge()
	repo.DeleteUser(id)
}

func main() {
	HandleDelete(&App{Services: &Services{Users: &UserService{repo: &UserRepository{}, notifier: SmsNotifier{}}}}, 1)
}

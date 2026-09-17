package main

type UnitOfWork struct {
	Users *UserRepository
	Audit AuditLog
}

type Box[T any] struct {
	Item T
}

type Folder struct {
	Parent *Folder
}

func (f *Folder) Root() *Folder {
	return f
}

func NewUnitOfWork() *UnitOfWork {
	return &UnitOfWork{Users: &UserRepository{}}
}

type Deps struct {
	uow *UnitOfWork
}

type Handler struct {
	*Deps
	box Box[*UserRepository]
}

func (h *Handler) DeleteAccount(id int, folder *Folder) {
	h.uow.Users.DeleteUser(id)
	h.Deps.uow.Audit.DeleteUser(id)
	h.box.Item.DeleteUser(id)
	users := AuditLog{}
	NewUnitOfWork().Users.DeleteUser(id)
	users.DeleteUser(id)
	folder.Parent.Parent.Parent.Parent.Parent.Root()
	folder.Parent.Parent.Parent.Parent.Parent.Parent.Root()
}

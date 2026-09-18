package main

import "example.com/fixture/store"

type Depot struct {
	People *UserRepository
}

type Source interface {
	Source() *UserRepository
}

func (d *Depot) PeopleRepo() *UserRepository {
	return d.People
}

func (d *Depot) Trail() (AuditLog, error) {
	return AuditLog{}, nil
}

func OpenDepot() *Depot {
	return &Depot{}
}

func twoDepots() (*Depot, *Depot) {
	return OpenDepot(), OpenDepot()
}

func First(depot *Depot, src Source, id int) {
	repo := depot.PeopleRepo()
	repo.DeleteUser(id)
	trail, _ := depot.Trail()
	trail.DeleteUser(id + 1)
	OpenDepot().People.DeleteUser(id + 2)
	store.Open().Close()
	sourced := src.Source()
	sourced.DeleteUser(id + 3)
	OpenDepot().PeopleRepo().DeleteUser(id + 4)
}

func Second(id int) {
	_, depot := twoDepots()
	repo := depot.PeopleRepo()
	repo.DeleteUser(id + 5)
}

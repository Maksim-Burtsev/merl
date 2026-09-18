package main

func LoadRepos() []*UserRepository {
	return nil
}

func Sweep(repos []*UserRepository, logs map[string]AuditLog, id int) {
	for _, repo := range repos {
		repo.DeleteUser(id)
	}
	for _, entry := range logs {
		entry.DeleteUser(id + 1)
	}
	loaded := LoadRepos()
	for _, repo := range loaded {
		repo.DeleteUser(id + 2)
	}
}

// One variable is an index or a key, and over a channel an element the rules do not read.
func Drain(queue chan *UserRepository, id int) {
	for repo := range queue {
		repo.DeleteUser(id + 3)
	}
}

// A slice made or written out in place says what it holds.
func Collect(id int) {
	made := make([]*UserRepository, 0, 2)
	for _, repo := range made {
		repo.DeleteUser(id + 4)
	}
	listed := []AuditLog{{}}
	for _, entry := range listed {
		entry.DeleteUser(id + 5)
	}
}

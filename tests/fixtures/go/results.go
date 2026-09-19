package main

// A named result and a parameter read as a local of the body does, `Reload.err` (#100).
func (r *UserRepository) Reload(id int) (user User, err error) {
	count := id
	if count == 0 {
		return user, err
	}
	return r.FindUser(id), nil
}

func ReloadPlain(id int) (err error) {
	return err
}

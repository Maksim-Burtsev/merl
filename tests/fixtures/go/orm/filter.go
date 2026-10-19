package orm

type Filter struct{}

func (f Filter) Or(query any) Filter {
	return f
}

func (f Filter) And(query any) Filter {
	return f
}

package store

// Session is what Open hands out.
type Session struct{}

// Open starts a session. The method of the same name below is not what `store.Open` names.
func Open() *Session {
	return &Session{}
}

func (s *Session) Open() bool {
	return true
}

func (s *Session) Close() {}

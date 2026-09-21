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

// SessionList is a named slice, read from another package by what it holds.
type SessionList []*Session

// SessionTwin is an alias, read where it is declared: `Session` means nothing to the importer.
type SessionTwin = Session

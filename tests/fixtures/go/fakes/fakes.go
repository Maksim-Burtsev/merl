package fakes

// Open stands in for store.Open in tests.
func Open() {}

// Stub is what NewStub hands out.
type Stub struct{}

func NewStub() *Stub {
	return &Stub{}
}

func (s *Stub) Wipe() {}

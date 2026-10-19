package orm

import "xorm.io/xorm"

type SQLSession interface {
	Join(op string) *xorm.Session
	Where(query any) *Query
}

type Engine interface {
	SQLSession
	Ping() error
}

type Query struct{}

func (q *Query) And(query any) *Query {
	return q
}

func GetEngine() Engine {
	return nil
}

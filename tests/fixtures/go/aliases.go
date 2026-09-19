package main

import "example.com/fixture/store"

// An alias is the type it names, through another alias and another package (#100).
type RepoTwin = UserRepository

type RepoAgain = RepoTwin

type SessionAlias = store.Session

// A defined type is a type of its own: `AuditKind.Flush` is not `auditFlusher.Flush`.
type auditFlusher struct{}

func (f auditFlusher) Flush() {}

type AuditKind auditFlusher

func (k AuditKind) Flush() {}

func AliasRotate(first *RepoTwin, again *RepoAgain, session *SessionAlias, kind AuditKind, twin *store.SessionTwin, id int) {
	first.DeleteUser(id + 20)
	again.DeleteUser(id + 21)
	session.Close()
	kind.Flush()
	twin.Close()
}

// Methods declared on the alias are found under its name, not skipped for what `auditBook`
// declares or embeds.
type auditBase struct{}

func (b auditBase) Seal() {}

type auditBook struct{ auditBase }

type AuditTwin = auditBook

func (t AuditTwin) Seal() {}

func AliasSeal(twin AuditTwin) {
	twin.Seal()
}

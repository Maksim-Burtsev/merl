import repos as models
from depot import Cover, Crate, Label, Session, Trail
from depot import FakeUsers, Hook, Pallet, Ring, Tray
from repos import AuditLog as Trail2, UserRepository as Users
from store import open_session as dial


def renamed(repo: Users, log: Trail2) -> None:
    repo.delete_user(1)
    log.delete_user(2)
    Users().find_user(3)


def through_a_module(repo: models.UserRepository) -> None:
    repo.delete_user(4)


def reexported(crate: Crate, cover: Cover, conn: Session, trail: Trail) -> None:
    crate.seal()
    cover.seal()
    conn.close()
    trail.delete_user(5)
    dial().close()


def starred(label: Label, users: FakeUsers) -> None:
    label.seal()
    users.users


def shadowed(Users: Trail2) -> None:
    Users.delete_user(7)


def not_handed_on(pallet: Pallet, tray: Tray, hook: Hook, ring: Ring) -> None:
    pallet.seal()
    tray.seal()
    hook.seal()
    ring.seal()

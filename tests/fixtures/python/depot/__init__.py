"""The depot package. A docstring imports nothing:

    from .crates import Hook
"""

from fakes import UserRepository as FakeUsers
from repos import AuditLog as Trail
from store import Session

from .crates import Crate
from .crates import Lid as Cover
from .labels import *

__all__ = ["Cover", "Crate", "FakeUsers", "Session", "Trail", "stamp"]


def stamp() -> None:
    pass

try:
    from .crates import Pallet
except ImportError:
    from .labels import Pallet

from .crates import Tray
from .loop import Ring

if stamp is None:
    Tray = Crate


def lazy() -> None:
    from .crates import Hook

    Hook().seal()

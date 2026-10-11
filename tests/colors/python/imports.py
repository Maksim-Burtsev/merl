#!/usr/bin/env python3
"""A module docstring: imports, constants and module-level names."""
from __future__ import annotations

import os
import os.path as osp
from collections import OrderedDict, defaultdict
from typing import TYPE_CHECKING, Any, TypeAlias, TypeVar, NewType
from . import sibling
from ..parent.child import Thing as Other

if TYPE_CHECKING:
    from pathlib import Path

__all__ = ["Order", "total"]
__version__ = "1.2.3"

MAX_ITEMS = 100
DEFAULT_NAME: str = "unnamed"
Number: TypeAlias = int | float
T = TypeVar("T")
UserId = NewType("UserId", int)
nothing = None
flags = (True, False, ...)
counts = defaultdict(int)
path = osp.join(os.getcwd(), "data")
print(__name__, __file__, NotImplemented)

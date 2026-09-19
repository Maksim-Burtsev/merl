from .crates import Crate


class Label(Crate):
    def seal(self) -> None:
        pass


class Pallet:
    def seal(self) -> None:
        pass

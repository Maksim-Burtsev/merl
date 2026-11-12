from cruets.cruet import Cruet


class Phial(Cruet):
    def uncork(self) -> None:
        pass

    def stopper(self) -> int:
        return 2

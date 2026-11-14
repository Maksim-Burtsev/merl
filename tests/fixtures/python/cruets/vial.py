from cruets.cruet import Cruet


class Vial(Cruet):
    def uncork(self) -> None:
        pass

    def stopper(self) -> int:
        return 1


class Flute(Vial):
    def uncork(self) -> None:
        pass

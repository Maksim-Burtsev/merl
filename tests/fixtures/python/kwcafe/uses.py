from kwcafe.brewer import kwbrew
from kwcafe.pot import Kwpot


def serve():
    mugful = 5
    pot = Kwpot()
    kwbrew(mugful=mugful)
    #      ^ d: kwcafe/brewer.py:1
    #             ^ d: kwcafe/uses.py:6
    pot.kwbrew(mugful=4)
    #          ^ d: kwcafe/pot.py:7
    kwbrew(mugful)
    #      ^ d: kwcafe/uses.py:6
    return pot

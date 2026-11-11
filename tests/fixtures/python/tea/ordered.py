from collections import OrderedDict

from tea.flasks import Flask


class Ledger[K, V](dict[K, V]):
    pass


# A subscripted mapping class is indexed by its value type, as on master (#1469).
def pour(by_name: OrderedDict[str, Flask], book: Ledger[str, Flask]) -> int:
    a = by_name["a"].tip()
    #                ^ d: tea/flasks.py:6
    return a + book["a"].tip()
    #                    ^ d: tea/flasks.py:6

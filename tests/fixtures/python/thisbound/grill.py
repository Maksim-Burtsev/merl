from thisbound.griddle import Griddle


class Grill:
    def sear(self):
        return 3

    def go(self):
        outer = self
        outer.sear()
        #     ^ d: thisbound/grill.py:5

        class Inner:
            def sear(self):
                return 4

            def run(this):
                outer.sear()
                #     ^ d: thisbound/grill.py:5
                this.sear()
                #    ^ d: thisbound/grill.py:14

            def near(this):
                outer = Griddle()
                outer.sear()
                #     ^ d: thisbound/griddle.py:2

        return Inner

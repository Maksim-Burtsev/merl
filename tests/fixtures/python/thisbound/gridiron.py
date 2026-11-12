class Gridiron:
    def sear(self):
        return 2

    def scoped(self, items):
        list(map(lambda i: self.sear(), items))
        #                       ^ d: thisbound/gridiron.py:2

        def inner():
            return self.sear()
            #           ^ d: thisbound/gridiron.py:2

        return inner

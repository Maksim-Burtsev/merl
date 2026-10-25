"""A local assigned in a method's body is its own declaration, never a fixture of its name (#1268)."""


def get_backend():
    return object()


class TestSearch:
    def test_ordering(self):
        backend = get_backend()
        #^ d: shop/rigs/test_search.py:10
        return backend

    def test_keyword(self):
        return make(
            backend=get_backend(),
            #^ d: shop/rigs/test_search.py:25
        )

    def test_fixture(self, backend):
        #                   ^ d: picker shop/rigs/conftest.py:5
        return backend


def make(backend=None):
    return backend

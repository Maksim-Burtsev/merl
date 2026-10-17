try:
    from tea.blends import *
except ImportError:
    pass


def serve() -> int:
    return infuse()
    #      ^ d: picker tea/blends.py:1, tea/brewing.py:106, tea/listed.py:4

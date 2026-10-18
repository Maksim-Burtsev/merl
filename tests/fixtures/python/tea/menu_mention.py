from tea.blends import *
from tea.mentions import *


def serve() -> int:
    return infuse() + _infuse()
    #      ^ d: picker tea/blends.py:1, tea/brewing.py:106, tea/listed.py:4
    #                 ^ d: picker tea/blends.py:5, tea/mentions.py:4

from unittest.mock import patch

from shop import quoted
from shop.quoted import Stylus


@patch("shop.quoted.Nib.blot")
#                       ^ d: shop/quoted.py:7
@patch('shop.quoted.Stylus.blot')
#                          ^ d: shop/quoted.py:2
@patch("shop.quoted.a")
#                   ^ d: shop/quoted.py:11
@patch("shop.quoted.Pen.blot")
#                       ^ d: none
@patch("shelf.quoted.Nib.blot")
#                        ^ d: none
@patch("kitchen.Nib")
#               ^ d: shop/quoted.py:6
@patch("quoted.Nib.blot")
#              ^ d: shop/quoted.py:6
def test_blot(s: Stylus) -> str:
    return "s.blot" + quoted.a(s)
    #         ^ d: none

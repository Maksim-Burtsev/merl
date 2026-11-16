from unittest.mock import patch

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
def test_blot(s: Stylus) -> str:
    return "s.blot"
    #         ^ d: none

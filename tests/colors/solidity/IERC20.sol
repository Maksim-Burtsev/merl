// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function balanceOf(address who) external view returns (uint256);
}

library SafeMath {
    function add(uint256 a, uint256 b) internal pure returns (uint256 c) {
        c = a + b;
        assert(c >= a);
    }
}

abstract contract Base {
    address payable public owner = payable(address(this));
    bytes4 constant SELECTOR = bytes4(keccak256("f()"));
    int128 private delta = -42;
    string text = unicode"hello";
    bytes hexed = hex"deadbeef";

    function f() public virtual;
}

type Price is uint128;

using SafeMath for uint256;

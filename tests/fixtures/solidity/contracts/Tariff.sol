// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

uint256 constant LIMIT = 10;

type Price is uint128;

interface Tariff {
    function rate(uint256 grams) external returns (uint256);
    function describe() external view returns (string memory);
}

abstract contract Ledger {
    mapping(address => uint256) private balances;
    address immutable owner;
    uint256[] public entries;

    enum Side { Buy, Sell }

    enum Zone {
        Local,
        Abroad
    }

    struct Parcel {
        uint256 grams;
        Zone zone;
    }

    modifier onlyKeeper {
        _;
    }

    function gross(uint256 net) public pure returns (uint256 total) {
        uint256 fee = net / 100;
        return net + fee;
    }
}

/**
 * A ledger keeps its entries:
 * contract Shadow is Ledger {
 * function shadowed() public {}
 * uint256 public shadowCount;
 */
/* event Ghost(uint256 id); */
// modifier phantom {

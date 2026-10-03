// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface Courier {
    function weigh(uint256 grams) external returns (uint256);
}

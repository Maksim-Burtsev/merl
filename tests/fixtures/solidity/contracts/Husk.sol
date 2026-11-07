// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Husk {
    string constant SAMPLE = "\
function huskQuoted() public {}\
function huskReal() public {}\
";
    function huskReal() public {}
    string constant DIR = "C:\\";
    function huskAfter() public {}

    function huskUse() public {
        huskQuoted();
//      ^ d: none
        huskReal();
//      ^ d: contracts/Husk.sol:9
        huskAfter();
//      ^ d: contracts/Husk.sol:11
    }
}

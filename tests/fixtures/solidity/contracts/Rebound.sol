// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

struct Crate {
    uint256 heft;
}

contract Rebound {
    uint256 tally;
    uint256 notch;
    event Counted(uint256 tally);

    function key(uint256 heft) external pure returns (Crate memory) {
        return Crate({heft: heft});
//                    ^ d: contracts/Rebound.sol:5
//                          ^ d: contracts/Rebound.sol:13
    }

    function eventKey(uint256 tally) external {
        emit Counted({tally: tally});
//                    ^ d: !jump
//                           ^ d: contracts/Rebound.sol:19
        emit Counted({
            tally: tally
//          ^ d: !jump
//                 ^ d: contracts/Rebound.sol:19
        });
    }

    function ternary(uint256 notch, bool c) external pure returns (uint256 x) {
        x = c ? notch : 1;
//              ^ d: contracts/Rebound.sol:30
        x = c ?
            notch : x;
//          ^ d: contracts/Rebound.sol:30
        assembly {
            notch := 1
//          ^ d: contracts/Rebound.sol:30
        }
    }

    function selfInit() external {
        uint256 notch = notch + 1;
//                      ^ d: picker contracts/Rebound.sol:10, …
        emit Counted(notch);
//                   ^ d: contracts/Rebound.sol:43
    }

    function tupleSelf() external {
        (uint256 a, uint256 notch) = (notch, 1);
//                                    ^ d: picker contracts/Rebound.sol:10, …
    }

    function loopSelf() external {
        for (uint256 tally = tally; tally < 3; tally++) {}
//                           ^ d: picker contracts/Rebound.sol:9, …
//                                  ^ d: contracts/Rebound.sol:55
    }

    function sameLine() external {
        uint256 notch = 1; emit Counted(notch);
//                                      ^ d: contracts/Rebound.sol:61
    }

    function readSelf() external {
        uint256 tally = tally; emit Counted(tally);
//                      ^ d: picker contracts/Rebound.sol:9, …
//                                          ^ d: contracts/Rebound.sol:66
    }

    function controls() external {
        emit Counted(notch);
//                   ^ d: picker contracts/Rebound.sol:10, …
        // uint256 notch = notch;
        emit Counted(notch);
//                   ^ d: picker contracts/Rebound.sol:10, …
        string memory s = "uint256 notch = notch;";
        emit Counted(notch);
//                   ^ d: picker contracts/Rebound.sol:10, …
    }
}

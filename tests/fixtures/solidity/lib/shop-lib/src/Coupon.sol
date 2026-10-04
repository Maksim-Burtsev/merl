// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

library Coupon {
    struct Voucher {
        uint256 rate;
        bool used;
    }

    function rate(Voucher memory v) internal pure returns (uint256) {
        return v.rate;
    }

    function describe(Voucher memory v) internal pure returns (string memory) {
        return v.used ? "used" : "fresh";
    }
}

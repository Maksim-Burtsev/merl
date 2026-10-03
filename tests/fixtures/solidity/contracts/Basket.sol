// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Tariff as Rate, Ledger} from "./Tariff.sol";
import "./Tariff.sol" as T;
import * as C from "@shop/Coupon.sol";
import {Coupon} from "@shop/Coupon.sol";
//                        ^ d: lib/shop-lib/src/Coupon.sol:1
import {Courier} from "@courier/Courier.sol";
import "./ShopToken.sol";
//        ^ d: contracts/ShopToken.sol:1

contract Basket is Ledger {
    using C.Coupon for C.Coupon.Voucher;
    Rate public tariff;
//  ^ d: contracts/Tariff.sol:8
//  status: Rate: via import ./Tariff.sol
    T.Price public price;
//    ^ d: contracts/Tariff.sol:6
    Courier public courier;
//  ^ d: lib/shop-lib/src/courier/Courier.sol:4
    T.Side public side = T.Side.Buy;
//                              ^ d: contracts/Tariff.sol:18
    T.Zone public zone = T.Zone.Abroad;
//                              ^ d: contracts/Tariff.sol:22

    function weigh(uint256 grams, C.Coupon.Voucher memory v) external onlyKeeper returns (uint256 left) {
//                                                                    ^ d: contracts/Tariff.sol:30
        uint256 total = gross(grams) + LIMIT + MAX_SUPPLY;
//                      ^ d: contracts/Tariff.sol:34
//                                     ^ d: contracts/Tariff.sol:4
//                                             ^ d: contracts/ShopToken.sol:12
        left = total - C.Coupon.rate(v);
//                              ^ d: lib/shop-lib/src/Coupon.sol:10
//      ^ d: contracts/Basket.sol:27
//             ^ d: contracts/Basket.sol:29
        courier.weigh(grams);
//      ^ d: contracts/Basket.sol:20
//                    ^ d: contracts/Basket.sol:27
        if (shadowCount == 0 && shadowed() && phantom) {
//          ^ d: none
//                              ^ d: none
//                                            ^ d: none
            revert Ghost(1);
//                 ^ d: none
        }
    }

    function settle(
        uint256 grams,
        Parcel memory parcel
    ) internal view returns (uint256 left) {
        uint256 fee = gross(grams) + entries.length;
//                          ^ d: contracts/Basket.sol:50
//                                   ^ d: contracts/Tariff.sol:16
        for (uint256 i = 0; i < 3; i++) {
            left += fee + i + parcel.grams + balances[owner];
//                  ^ d: contracts/Basket.sol:53
//                        ^ d: contracts/Basket.sol:56
//                            ^ d: contracts/Basket.sol:51
//                                   ^ d: contracts/Tariff.sol:26
//                                           ^ d: contracts/Tariff.sol:14
//                                                    ^ d: contracts/Tariff.sol:15
//          ^ d: contracts/Basket.sol:52
        }
        Side s = Side.Sell;
//               ^ d: contracts/Tariff.sol:18
//                    ^ d: contracts/Tariff.sol:18
        Coupon.Voucher memory v;
//      ^ d: lib/shop-lib/src/Coupon.sol:4
//             ^ d: lib/shop-lib/src/Coupon.sol:5
    }

    receive() external payable {}
}

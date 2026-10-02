// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
//                                                       ^ d: node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol:1
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract ShopToken is ERC20, Ownable {
//                    ^ d: node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol:4
//                    status: ERC20: via import @openzeppelin/contracts/token/ERC20/ERC20.sol
//                           ^ d: node_modules/@openzeppelin/contracts/access/Ownable.sol:4
    uint256 public constant MAX_SUPPLY = 1_000_000e18;

    event Minted(address indexed to, uint256 amount);

    error SupplyExceeded(uint256 requested);

    constructor() ERC20("Shop", "SHOP") Ownable(msg.sender) {}

    modifier withinSupply(uint256 amount) {
        if (totalSupply() + amount > MAX_SUPPLY) revert SupplyExceeded(amount);
//                                   ^ d: contracts/ShopToken.sol:12
//                                                      ^ d: contracts/ShopToken.sol:16
//          ^ d: node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol:9
        _;
    }

    function mint(address to, uint256 amount) external onlyOwner withinSupply(amount) {
//                                                     ^ d: node_modules/@openzeppelin/contracts/access/Ownable.sol:7
//                                                               ^ d: contracts/ShopToken.sol:20
//                                                               status: withinSupply → ShopToken.withinSupply (by name, 1 match)
        _mint(to, amount);
//      ^ d: node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol:13
        emit Minted(to, amount);
//           ^ d: contracts/ShopToken.sol:14
    }
}

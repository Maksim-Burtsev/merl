// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "./IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title A simple token
/// @notice Holds balances and moves them.
contract Token is IERC20, Ownable {
    using SafeMath for uint256;

    struct Account {
        uint256 balance;
        mapping(address => uint256) allowances;
    }

    enum Status { Active, Frozen }

    event Transfer(address indexed from, address indexed to, uint256 value);
    error InsufficientBalance(uint256 available, uint256 required);

    string public constant NAME = "Token";
    uint8 public immutable decimals;
    uint256 private _totalSupply = 1_000_000 * 10 ** 18;
    mapping(address => Account) internal accounts;
    Status public status = Status.Active;

    modifier onlyActive() {
        require(status == Status.Active, "frozen");
        _;
    }

    constructor(uint8 decimals_) Ownable(msg.sender) {
        decimals = decimals_;
        accounts[msg.sender].balance = _totalSupply;
    }

    /**
     * @dev Moves `amount` tokens to `to`.
     */
    function transfer(address to, uint256 amount) external override onlyActive returns (bool) {
        Account storage from = accounts[msg.sender];
        if (from.balance < amount) {
            revert InsufficientBalance({available: from.balance, required: amount});
        }
        unchecked {
            from.balance -= amount;
        }
        accounts[to].balance += amount;
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    function balanceOf(address who) public view virtual returns (uint256) {
        return accounts[who].balance;
    }

    function hash(bytes calldata data) external pure returns (bytes32 h) {
        h = keccak256(abi.encodePacked(data, uint256(0x1f), true, 1 ether, 2 days));
        assembly {
            let x := mload(0x40)
            mstore(x, h)
        }
    }

    receive() external payable {}

    fallback() external payable {
        for (uint256 i = 0; i < 3; i++) {
            if (i == 2) break;
            else continue;
        }
        while (false) {}
        delete accounts[address(0)];
        try this.balanceOf(msg.sender) returns (uint256 b) {
            b;
        } catch Error(string memory reason) {
            reason;
        }
    }
}

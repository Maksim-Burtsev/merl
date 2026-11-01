// Every construct `f` folds in Solidity, and the cases that once broke it.  // f: none
pragma solidity ^0.8.20;  // f: none

import {  // f: none
    A,  // f: none
    B  // f: none
} from "./x.sol";  // f: none
import "./y.sol";  // f: none

/**  // f: 10-12
 * a doc comment  // f: 10-12
 */  // f: 10-12
error Free(  // f: 13-16
    uint256 a,  // f: 13-16
    uint256 b  // f: 13-16
);  // f: 13-16

struct Point {  // f: 18-21
    uint256 x;  // f: 18-21
    uint256 y;  // f: 18-21
}  // f: 18-21

enum Color {  // f: 23-26
    Red,  // f: 23-26
    Green  // f: 23-26
}  // f: 23-26

using {  // f: none
    add,  // f: none
    sub  // f: none
} for Point global;  // f: none

function add(Point memory a, Point memory b) pure returns (Point memory) {  // f: 33-38
    return Point({  // f: 34-37
        x: a.x + b.x,  // f: 33-38
        y: a.y + b.y  // f: 33-38
    });  // f: 33-38
}  // f: 33-38

function sub(  // f: none
    Point memory a,  // f: none
    Point memory b  // f: none
) pure returns (Point memory) {  // f: 43-45
    return Point(a.x - b.x, a.y - b.y);  // f: 43-45
}  // f: 43-45

abstract contract Base is  // f: none
    A,  // f: none
    B  // f: 50-191
{  // f: 50-191
    event Moved(  // f: 51-54
        address indexed who,  // f: 50-191
        uint256 amount  // f: 50-191
    ) anonymous;  // f: 50-191

    mapping(address => mapping(uint256 => bool)) private _seen;  // f: 50-191

    uint256[] private _list = [  // f: 58-61
        1,  // f: 50-191
        2  // f: 50-191
    ];  // f: 50-191

    bytes32 private constant KEY =  // f: 50-191
        keccak256(  // f: 64-66
            "key"  // f: 50-191
        );  // f: 50-191

    modifier only(  // f: 50-191
        address who  // f: 50-191
    ) {  // f: 70-76
        require(  // f: 71-74
            msg.sender == who,  // f: 70-76
            "no"  // f: 70-76
        );  // f: 70-76
        _;  // f: 70-76
    }  // f: 70-76

    constructor(uint256 a) A(  // f: 50-191
        a  // f: 50-191
    ) B() {  // f: 80-82
        _list.push(a);  // f: 80-82
    }  // f: 80-82

    receive() external payable {  // f: 84-89
        emit Moved(  // f: 85-88
            msg.sender,  // f: 84-89
            msg.value  // f: 84-89
        );  // f: 84-89
    }  // f: 84-89

    fallback() external {  // f: 91-96
        revert Free(  // f: 91-96
            1,  // f: 91-96
            2  // f: 91-96
        );  // f: 91-96
    }  // f: 91-96

    function f(uint256 n) public virtual returns (uint256 r, uint256 s) {  // f: 98-186
        if (n == 0) {  // f: 99-105
            return (1, 2);  // f: 98-186
        } else if (n == 1) {  // f: 101-102
            r = 3;  // f: 98-186
        } else {  // f: 103-105
            s = 4;  // f: 98-186
        }  // f: 98-186
        if (n > 5)  // f: 106-109
            r = 6;  // f: 98-186
        else  // f: 98-186
            s = 7;  // f: 98-186
        for (uint256 i = 0; i < n; i++) {  // f: 110-112
            r += i;  // f: 98-186
        }  // f: 98-186
        while (n > 0) {  // f: 113-115
            n--;  // f: 98-186
        }  // f: 98-186
        do {  // f: 116-118
            n++;  // f: 98-186
        } while (n < 3);  // f: 98-186
        unchecked {  // f: 119-121
            r++;  // f: 98-186
        }  // f: 98-186
        {  // f: 122-124
            s++;  // f: 98-186
        }  // f: 98-186
        try this.g(  // f: 125-126
            n  // f: 98-186
        ) returns (uint256 v) {  // f: 127-128
            r = v;  // f: 98-186
        } catch Error(string memory why) {  // f: 129-130
            s = 0;  // f: 98-186
        } catch (bytes memory) {  // f: 131-138
            s = 1;  // f: 98-186
        }  // f: 98-186
        (bool ok, ) = address(this).call{  // f: 134-136
            value: 0  // f: 98-186
        }(  // f: 98-186
            ""  // f: 98-186
        );  // f: 98-186
        (r, s) = (  // f: 139-142
            s,  // f: 98-186
            r  // f: 98-186
        );  // f: 98-186
        uint256[2] memory pair = [  // f: 143-146
            uint256(1),  // f: 98-186
            2  // f: 98-186
        ];  // f: 98-186
        r = ok  // f: 98-186
            ? pair[0]  // f: 98-186
            : uint256(  // f: 98-186
                pair[1]  // f: 98-186
            );  // f: 98-186
        r = uint256(  // f: 98-186
            s  // f: 98-186
        );  // f: 98-186
        r = abi.decode(  // f: 155-158
            "",  // f: 98-186
            (uint256)  // f: 98-186
        );  // f: 98-186
        string memory t = string.concat(  // f: 159-162
            "a",  // f: 98-186
            "b"  // f: 98-186
        );  // f: 98-186
        /* a block  // f: 163-164
           comment */  // f: 98-186
        assembly ("memory-safe") {  // f: 165-185
            let x := mload(  // f: 98-186
                0x40  // f: 98-186
            )  // f: 98-186
            if iszero(x) {  // f: 169-171
                revert(0, 0)  // f: 98-186
            }  // f: 98-186
            for { let i := 0 } lt(i, 2) { i := add(i, 1) } {  // f: 172-174
                x := add(x, i)  // f: 98-186
            }  // f: 98-186
            switch x  // f: 98-186
            case 0 {  // f: 176-178
                x := 1  // f: 98-186
            }  // f: 98-186
            default {  // f: 179-181
                x := 2  // f: 98-186
            }  // f: 98-186
            function h(a) -> b {  // f: 182-184
                b := a  // f: 98-186
            }  // f: 98-186
        }  // f: 98-186
    }  // f: 98-186

    function g(uint256 n) external pure returns (uint256) {  // f: 188-190
        return n;  // f: 188-190
    }  // f: 188-190
}  // f: 50-191

interface I {  // f: 193-199
    function a() external;  // f: 193-199

    function b(  // f: 193-199
        uint256 x  // f: 193-199
    ) external returns (uint256);  // f: 193-199
}  // f: 193-199

library L {  // f: 201-205
    function c() internal pure returns (uint256) {  // f: 202-204
        return type(uint256).max;  // f: 202-204
    }  // f: 202-204
}  // f: 201-205

contract C {  // f: 207-275
    string private s = "a { b // c";  // f: 207-275
    string private u = unicode"é } /*";  // f: 207-275
    bytes private h = hex"00ff";  // f: 207-275

    struct S {  // f: 212-215
        mapping(address => uint256) m;  // f: 207-275
        uint256[] list;  // f: 207-275
    }  // f: 207-275

    event E();  // f: 207-275
    error X();  // f: 207-275

    function f(  // f: 230-245
        uint256 a // a {  // f: 230-245
    )  // f: 230-245
        public  // f: 230-245
        virtual  // f: 230-245
        override(  // f: 230-245
            A,  // f: 230-245
            B  // f: 230-245
        )  // f: 230-245
        returns (uint256)  // f: 230-245
    {  // f: 230-245
        if (  // f: 231-234
            a > 1 &&  // f: 230-245
            a < 3  // f: 230-245
        ) revert X();  // f: 230-245
        if (a == 2) revert(  // f: 235-237
            "two"  // f: 230-245
        );  // f: 230-245
        emit I.E(  // f: 238-240
            a  // f: 230-245
        );  // f: 230-245
        return  // f: 230-245
            this.g(  // f: 242-244
                a  // f: 230-245
            );  // f: 230-245
    }  // f: 230-245

    function g(uint256 a) external returns (uint256) {  // f: 247-267
        assembly {  // f: 248-253
            {  // f: 249-251
                let y := 1  // f: 247-267
            }  // f: 247-267
            let z := 2  // f: 247-267
        }  // f: 247-267
        uint256 b = a > 1  // f: 247-267
            ? a  // f: 247-267
            : 2;  // f: 247-267
        b = b +  // f: 257-260
            a * this.g(  // f: 247-267
                1  // f: 247-267
            );  // f: 247-267
        b = (b + 1) *  // f: 261-262
            this.g(2);  // f: 247-267
        bool x = !a.isZero(  // f: 263-265
            1  // f: 247-267
        );  // f: 247-267
        return b;  // f: 247-267
    }  // f: 247-267
    /* one line */  // f: 207-275
    function k() external {  // f: 269-274
        // comment {  // f: 269-274
        uint256 c = 1; /* two  // f: 271-272
        lines */  // f: 269-274
        c += 1;  // f: 269-274
    }  // f: 269-274
}  // f: 207-275

contract D {  // f: 277-292
    function f() {  // f: 278-291
        (address r, RecoverError error, bytes32 e) = tryRecover(  // f: 279-282
            h,  // f: 278-291
            s  // f: 278-291
        );  // f: 278-291
        if (a)  // f: 283-290
            if (b) {  // f: 284-285
                c();  // f: 278-291
            } else {  // f: 286-288
                d();  // f: 278-291
            }  // f: 278-291
        else  // f: 278-291
            e();  // f: 278-291
    }  // f: 278-291
}  // f: 277-292

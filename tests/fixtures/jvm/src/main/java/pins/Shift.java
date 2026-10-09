package pins;

import shop.basket.Gear;
import org.other.Shim;
//               ^ d: none

class Shift {
    Object pick() {
        return Gear.BRISK;
        //          ^ d: none
    }
}

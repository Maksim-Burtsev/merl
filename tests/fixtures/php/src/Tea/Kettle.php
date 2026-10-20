<?php

namespace Shop\Tea;

class Kettle extends Vessel
{
    public const SIZE = 3;

    public int $lid = 0;

    public function pour(int $cups): int
    {
        return $cups;
    }

    public function decant(): int
    {
        return $this->pour(1) + parent::rinse() + static::SIZE;
        //            ^ d: src/Tea/Kettle.php:11
        //                              ^ d: src/Tea/Vessel.php:14
        //                                                ^ d: src/Tea/Kettle.php:7
    }

    public function card(Kettle $other): string
    {
        $lid = $this->lid;
        $card = <<<CARD
            Lift the lid, then pour.
            //       ^ d: none
            //                 ^ d: none
            Hello {lid}.
            //     ^ d: none
            The lid is {$this->lid}, poured {$this->pour(1)}, others' $other->lid or ${lid} or $lid.
            //  ^ d: none
            //                 ^ d: src/Tea/Kettle.php:9
            //                                      ^ d: src/Tea/Kettle.php:11
            //                                                                ^ d: src/Tea/Kettle.php:9
            //                                                                                  ^ d: src/Tea/Kettle.php:26
            //                                                                         ^ d: src/Tea/Kettle.php:26
            Escaped \$lid stays text.
            //        ^ d: none
            CARD;
        return $card . <<<'NOTE'
            A note on the lid.
            //            ^ d: none
            NOTE;
    }
}

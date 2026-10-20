<?php

namespace Shop\Tea\Infuse;

use DateTimeImmutable;
use Shop\Tea\Infuse\{Leaf, Oolong as Dark};

final class Ceremony
{
    public function outside(): string
    {
        $when = new DateTimeImmutable();
        $items = new \ArrayObject([]);
        return $when->format('c') . $items->count();
        //            ^ d: none
        //                                  ^ d: none
    }

    public function chain(): int
    {
        return (new Leaf())
            ->withWater(80)
            //^ d: src/Tea/Infuse/Leaf.php:22
            ->steep()
            //^ d: src/Tea/Infuse/Leaf.php:27
            ->pour();
            //^ d: picker src/Tea/Infuse/Gaiwan.php:7, src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Infuse/Gaiwan.php:7 (#1174)
    }

    public function callOnCall(Leaf $leaf): int
    {
        return $leaf->steep()->rinse()->pour();
        //                     ^ d: picker src/Tea/Infuse/Gaiwan.php:12, src/Tea/Vessel.php:14; want src/Tea/Infuse/Gaiwan.php:12 (#1174)
        //                              ^ d: picker src/Tea/Infuse/Gaiwan.php:7, src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Infuse/Gaiwan.php:7 (#1174)
    }

    public function shadow(Leaf $cup): int
    {
        $cup = $cup->steep();
        //           ^ d: src/Tea/Infuse/Leaf.php:27
        return $cup->pour();
        //           ^ d: picker src/Tea/Infuse/Gaiwan.php:7, src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Infuse/Gaiwan.php:7 (#1174)
    }

    public function destructure(): int
    {
        [$first, $second] = [new Leaf(), new Gaiwan()];
        return $second->pour() + $first->count();
        //              ^ d: picker src/Tea/Infuse/Gaiwan.php:7, src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Infuse/Gaiwan.php:7 (#1175)
        //                               ^ d: picker src/Rules/Meter.php:7, src/Tea/Infuse/Leaf.php:17; want src/Tea/Infuse/Leaf.php:17 (#1175)
    }

    public function loop(array $leaves): string
    {
        $out = '';
        foreach ($leaves as $leaf) {
            $leaf = new Dark();
            $out .= $leaf->grade();
            //             ^ d: src/Tea/Infuse/Oolong.php:12
        }
        return $out;
    }

    public function inherited(): string
    {
        $dark = new Dark();
        //          ^ d: src/Tea/Infuse/Oolong.php:5
        return $dark->format() . $dark->grade() . $dark->shape() . $dark->roastShape();
        //            ^ d: src/Tea/Infuse/Leaf.php:12
        //                              ^ d: src/Tea/Infuse/Oolong.php:12
        //                                               ^ d: src/Tea/Infuse/Rolled.php:7
        //                                                                ^ d: src/Tea/Infuse/Roasted.php:7
    }

    public function viaInterface(Graded $graded): string
    {
        return $graded->grade();
        //              ^ d: src/Tea/Infuse/Graded.php:7
    }

    public function anonymous(): string
    {
        $anon = new class extends Leaf {
            public function grade(): string
            {
                return 'anon';
            }
        };
        return $anon->grade() . $anon->format();
        //            ^ d: src/Tea/Infuse/Ceremony.php:84
        //                             ^ d: src/Tea/Infuse/Leaf.php:12
    }

    public function closures(Leaf $leaf): string
    {
        $arrow = fn () => $leaf->grade();
        //                       ^ d: src/Tea/Infuse/Leaf.php:7
        $pour = $leaf->steep(...);
        //             ^ d: src/Tea/Infuse/Leaf.php:27
        return $arrow() . $pour()->pour();
        //                         ^ d: picker src/Tea/Infuse/Gaiwan.php:7, src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Infuse/Gaiwan.php:7 (#1174)
    }

    public function flush(): string
    {
        return Flush::First->label() . Flush::from('second')->label();
        //            ^ d: src/Tea/Infuse/Flush.php:7
        //                   ^ d: src/Tea/Infuse/Flush.php:10
        //                                    ^ d: none
        //                                                    ^ d: src/Tea/Infuse/Flush.php:10
    }

    public function conditional(): int
    {
        return warm_cup();
        //     ^ d: src/Tea/Infuse/helpers.php:6
    }

    public function strings(): string
    {
        $code = "class Leaf {} function warm_cup() {}";
        //             ^ d: src/Tea/Infuse/Leaf.php:5
        # function pour(): int
        return $code . <<<PHP
            public function shape(): string
            //              ^ d: none
            PHP;
    }
}

<?php

namespace Shop\Tea;

class Rebound
{
    public function sameLine(Kettle $pot): int
    {
        $cups = 1; $pot = new Teapot(); return $pot->pour($cups);
        //                                           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function chained(Kettle $pot): int
    {
        $cup = $pot = new Teapot(); return $pot->pour(1);
        //                                       ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function ownRightSide(Kettle $pot): int
    {
        $pot = $pot->pour(1);
        //           ^ d: src/Tea/Kettle.php:11
        return $pot;
    }

    public function coalesced(?Kettle $pot): int
    {
        $pot ??= new Teapot();
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function narrowedIf(Kettle $pot): int
    {
        if ($pot instanceof Teapot) {
            return $pot->pour(1);
        //               ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
        }
        return 0;
    }

    public function narrowedGuard(Kettle $pot): int
    {
        if (!$pot instanceof Teapot) {
            return 0;
        }
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function narrowedSame(Kettle $pot): int
    {
        if ($pot instanceof Kettle) {
            return $pot->pour(1);
        //               ^ d: src/Tea/Kettle.php:11
        }
        return 0;
    }

    public function uncaptured(Kettle $pot): int
    {
        $pour = function () { return $pot->pour(1); };
        //                                 ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
        return $pour();
    }

    public function captured(Kettle $pot): int
    {
        $pour = function () use ($pot) { return $pot->pour(1); };
        //                                            ^ d: src/Tea/Kettle.php:11
        return $pour();
    }

    public function byReference(Kettle $pot): int
    {
        $refill = function () use (&$pot) {
            $pot = new Teapot();
        };
        $refill();
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function extracted(Kettle $pot, array $vars): int
    {
        extract($vars);
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function variableVariable(Kettle $pot): int
    {
        ${'pot'} = new Teapot();
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function outParameter(Kettle $pot): int
    {
        preg_match('/pot/', 'teapot', $pot);
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function keyed(Kettle $pot, array $pots): int
    {
        ['pot' => $pot] = $pots;
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function wrappedForeach(Kettle $pot, array $pots): int
    {
        foreach (
            $pots as $pot
        ) {
            return $pot->pour(1);
        //               ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
        }
        return 0;
    }

    public function plainReads(Kettle $pot, int $m): int
    {
        $s = str_replace((string) $pot->lid, '', 'x'); $pot->pour(1);
        //                                                   ^ d: src/Tea/Kettle.php:11
        if ($m & $pot->lid && $pot->pour(1)) {}
        //                          ^ d: src/Tea/Kettle.php:11
        $ok = preg_match('/a/', (string) $pot->lid); $pot->pour($ok);
        //                                                 ^ d: src/Tea/Kettle.php:11
        $pot->pour(1); // $pot = new Teapot();
        //    ^ d: src/Tea/Kettle.php:11
        $s = '$pot = new Teapot();'; $pot->pour(1);
        //                                 ^ d: src/Tea/Kettle.php:11
        $same = $pot == null; $pot->pour(1);
        //                          ^ d: src/Tea/Kettle.php:11
        $this->fill(pot: 1); $pot->pour(1);
        //                         ^ d: src/Tea/Kettle.php:11
        $a = ['pot' => $pot, 'lid' => $pot->lid]; $pot->pour(1);
        //                                              ^ d: src/Tea/Kettle.php:11
        $pot->lid = 2; $pot->pour(1);
        //                   ^ d: src/Tea/Kettle.php:11
        $n = $pot->pour(1); $pot = new Teapot();
        //         ^ d: src/Tea/Kettle.php:11
        return $n + $same + count($a) + strlen($s);
    }

    public function fill(int $pot): int
    {
        return $pot;
    }

    public function counted(Kettle $pot): int
    {
        str_replace('a', 'b', 'c', $pot);
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }

    public function aliased(Kettle $pot): int
    {
        $alias = &$pot;
        $alias = new Teapot();
        return $pot->pour(1);
        //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Teapot.php:12, …
    }
}

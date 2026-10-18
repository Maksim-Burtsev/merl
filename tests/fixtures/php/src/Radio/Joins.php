<?php

namespace Shop\Radio;

use Illuminate\Database\Query\JoinClause;

class Joins
{
    public function on(string $event): void
    {
    }

    public function joined(): void
    {
        $this->leftJoin('t', function (JoinClause $join): void {
            $join->on('a', 'b');
            //     ^ d: none
        });
    }
}

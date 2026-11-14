<?php

namespace Shop\Thisbound;

class Skillet
{
    public function sear(): int
    {
        return 3;
    }

    public function aliased(): array
    {
        $self = $this;
        $h = function () use ($self) {
            return $self->sear();
            //            ^ d: src/Thisbound/Skillet.php:7
        };
        $direct = $self->sear();
        //                ^ d: src/Thisbound/Skillet.php:7
        $param = function ($self) {
            return $self->sear();
            //            ^ d: !jump
        };
        return [$h, $direct, $param];
    }

    public function anonymous(): array
    {
        $own = new class {
            public function sear(): int
            {
                return 4;
            }

            public function run(): int
            {
                return $this->sear();
                //            ^ d: src/Thisbound/Skillet.php:31
            }
        };
        $inherited = new class extends Griddle {
            public function run(): int
            {
                return $this->sear();
                //            ^ d: src/Thisbound/Griddle.php:7
            }
        };
        $after = $this->sear();
        //              ^ d: src/Thisbound/Skillet.php:7
        return [$own, $inherited, $after];
    }
}

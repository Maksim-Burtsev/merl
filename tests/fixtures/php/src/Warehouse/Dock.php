<?php

namespace Shop\Warehouse;

class Dock
{
    protected string $cargo = '';

    public function run(): void
    {
        $manifest = ['a' => 1];
        $berth = 'x';
    }

    public function moor(string $event): void
    {
    }
}

function hoist(): string
{
    return 'up';
}

<?php

namespace App;

class Basket
{
    private array $items = [];

    public function append(string $item): void
    {
        $this->items[] = $item;
    }

    public function count(): int
    {
        return count($this->items);
    }
}

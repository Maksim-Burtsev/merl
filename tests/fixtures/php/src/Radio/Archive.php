<?php

namespace Shop\Radio;

class Archive
{
    private Podcast $latest;

    public function find(): ?Podcast
    {
        return $this->latest;
    }

    public function subscribers(): int
    {
        return 0;
    }
}

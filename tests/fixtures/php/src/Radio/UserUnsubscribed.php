<?php

namespace Shop\Radio;

class UserUnsubscribed
{
    public function __construct(
        public readonly Podcast $podcast,
    ) {
    }
}

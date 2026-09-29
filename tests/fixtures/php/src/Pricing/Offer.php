<?php

namespace Shop\Pricing;

enum Offer: string
{
    case Plain = 'plain';
    case Cut = 'cut';

    const DEFAULT = self::Plain;
}

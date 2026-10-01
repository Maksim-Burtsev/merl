<?php

namespace Billing;

abstract class Document
{
    public const PAPER = 'A4';

    protected function render(string $format): string
    {
        return "{$format}:" . static::PAPER;
    }
}

trait Numbered
{
    protected function number(): string
    {
        return sprintf('INV-%05d', $this->id);
    }
}

final class Receipt
{
    public function render(string $format): string
    {
        return "receipt:{$format}";
    }

    public function number(): string
    {
        return 'R-1';
    }
}

final class Invoice extends Document
{
    use Numbered;

    public function __construct(private readonly int $id)
    {
    }

    public function pdf(): string
    {
        return $this->number() . ' ' . $this->render('pdf');
    }
}

function send_receipt(Receipt $receipt): string
{
    return $receipt->number();
}

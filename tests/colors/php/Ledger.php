<?php

declare(strict_types=1);

namespace App\Books;

use InvalidArgumentException;
use App\Contracts\{Countable, Handler};

/**
 * A ledger keeps entries in order.
 *
 * @param string $owner who keeps it
 */
#[Attribute]
final class Ledger implements Countable, \IteratorAggregate
{
    use HasEvents;

    public const LIMIT = 1_000;
    private const PATTERN = '/[a-z]+\d/';

    private array $entries = [];
    protected static int $count = 0;

    public function __construct(
        public readonly string $owner,
        private ?Handler $handler = null,
    ) {
        if (trim($owner) === '') {
            throw new InvalidArgumentException("owner is blank");
        }
        self::$count++;
    }

    public static function empty(): static
    {
        return new static('nobody');
    }

    public function getIterator(): \ArrayIterator
    {
        return new \ArrayIterator($this->entries);
    }

    public function count(): int
    {
        return count($this->entries);
    }

    public function total(): float
    {
        return array_sum(array_map(fn ($e) => $e['amount'], $this->entries));
    }

    public function add(float $amount, ?string $note = null): array
    {
        $entry = ['id' => count($this->entries), 'amount' => $amount, 'note' => $note];
        $this->entries[] = $entry;
        $this->handler?->moved($entry);
        return $entry;
    }

    public function sync(int ...$ids): void
    {
        foreach ($ids as $id) {
            if ($id < 0 || $id > self::LIMIT) {
                continue;
            }
            echo "synced {$id} of {$this->count()}\n";
        }
    }

    public function kind(array $entry): string
    {
        return match (true) {
            $entry['amount'] < 0 => 'debit',
            $entry['amount'] == 0 => throw new \LogicException('zero'),
            default => 'credit',
        };
    }

    public function report(): string
    {
        $lines = [];
        foreach ($this->entries as $i => $e) {
            $lines[] = sprintf('%-10s %8.2f', $e['note'] ?? '-', abs($e['amount']));
        }
        $raw = <<<TEXT
            total: {$this->total()}
            count: $i
            TEXT;
        return implode("\n", $lines) . $raw;
    }

    public function check(mixed $x): bool
    {
        $big = 0xFF_FF;
        $ratio = 1.5e3;
        return is_string($x) && strlen($x) > 0 || $big > 0 && $ratio !== 0.0 && preg_match(self::PATTERN, 'a1');
    }
}

$ledger = Ledger::empty();
try {
    $ledger->add(-2.5);
} catch (\LogicException $e) {
    echo $e->getMessage();
} finally {
    // done
}
?>
<ul>
  <?php foreach ($ledger as $e): ?>
    <li class="entry"><?= htmlspecialchars((string) $e['note']) ?></li>
  <?php endforeach; ?>
</ul>

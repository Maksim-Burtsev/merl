<?php

namespace Shop\Radio;

use PhanAn\Poddle\Values\ChannelMetadata;

/**
 * @property ChannelMetadata $metadata
 * @property ?string $author
 */
class Feed
{
    public function credits(Feed $other): array
    {
        return [$this->metadata->author, $other->metadata->author, $this->author];
        //                       ^ d: none
        //                                                ^ d: none
        //                                                                ^ d: src/Radio/Feed.php:9
    }

    /** @param Feed $source */
    public function __construct(
        private readonly ChannelMetadata $source,
    ) {
    }

    public function relay(): array
    {
        return $this->source->credits($this);
        //                    ^ d: src/Radio/Feed.php:13
        //                     status: via $this->source: Feed
    }
}

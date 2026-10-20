<?php

namespace Shop\Radio;

use Illuminate\Support\Collection;

/**
 * @property ?Artist $artist
 * @property Album|null $album
 * @property Artist|Album $credit
 * @property Collection<Artist> $artists
 * @property string $name
 */
class Song
{
    public function describe(Song $song): string
    {
        return $song->artist->name;
        //                    ^ d: src/Radio/Artist.php:6
        //                     status: via $song: Song → artist: Artist
    }
}

<?php

namespace Shop\Radio;

class Playlist
{
    public function covers(Song $song): void
    {
        $song->album->name;
        //            ^ d: src/Radio/Album.php:6
        $song->album->artist->debut->name;
        //                    ^ d: src/Radio/Artist.php:7
        //                           ^ d: src/Radio/Album.php:6
        $song->credit->name;
        //             ^ d: picker src/Radio/Album.php:6, src/Radio/Artist.php:6, …
        $song->artists->name;
        //              ^ d: picker src/Radio/Album.php:6, src/Radio/Artist.php:6, …
    }
}

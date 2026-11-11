<?php
function feast() {
    $c = new Kylix();
    $c->libate();
    //  ^ d: picker src/Kylix/Kylix.php:2, src/Kylix/Vessels.php:2, src/Kylix/Vessels.php:3, src/Kylix/Vessels.php:6; want src/Kylix/Kylix.php:2 (#1486)
    $s = new Skyphos();
    $s->libate();
    //  ^ d: picker src/Kylix/Kylix.php:2, src/Kylix/Vessels.php:2, src/Kylix/Vessels.php:3, src/Kylix/Vessels.php:6; want src/Kylix/Vessels.php:2 (#1486)
    $r = new Rhyton();
    $r->libate();
    //  ^ d: picker src/Kylix/Kylix.php:2, src/Kylix/Vessels.php:2, src/Kylix/Vessels.php:3, src/Kylix/Vessels.php:6; want src/Kylix/Vessels.php:3 (#1486)
    $k = new Kantharos();
    $k->libate();
    //  ^ d: src/Kylix/Vessels.php:6
}

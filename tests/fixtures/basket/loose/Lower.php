<?php

namespace Loose;

use loose\other\Route;

// #1452: PHP names ignore case, so this `use` names the project's `Loose\Other\Route`, never
// a package that is not installed.
final class Lower
{
    public function make(): Route
    {
        return new Route();
        //         ^ d: loose/Other/Route.php:5
    }
}

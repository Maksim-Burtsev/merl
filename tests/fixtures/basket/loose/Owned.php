<?php

namespace Loose;

use Loose\Other\Route;

// #1452: with no composer.json, a project class imported under its own full name still jumps.
final class Owned
{
    public function make(): Route
    {
        $route = new Route();
        //           ^ d: loose/Other/Route.php:5
        return Route::draw();
        //     ^ d: loose/Other/Route.php:5
        //            ^ d: loose/Other/Route.php:7
    }
}

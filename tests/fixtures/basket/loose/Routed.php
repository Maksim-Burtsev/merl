<?php

namespace Loose;

use Symfony\Component\Routing\Annotation\Route;

// #1452: no composer.json above this file, yet the `Route` of a package that is not in vendor/
// is the import's, never the namesake in `Other/Route.php`, which this file does not use.
#[Route('/x')]
//^ d: loose/Routed.php:5
//status: via import Symfony/Component/Routing/Annotation/Route (not installed)
final class Routed
{
    public function make(): Route
    {
        return new Route('/x');
        //         ^ d: loose/Routed.php:5
    }
}

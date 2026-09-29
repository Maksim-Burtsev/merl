<?php

namespace Shop;

# loads lib/* (#488)
function belowTheGlob(): int { return 1; }

function callsBelowTheGlob(): int
{
    return belowTheGlob();
    //     ^ d: src/Paths.php:6
}

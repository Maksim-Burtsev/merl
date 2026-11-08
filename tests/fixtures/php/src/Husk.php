<?php

$sample = '
function husk_quoted() {}
function husk_real() {}
';
$page = "
class HuskDouble {}
";
function husk_real() {}

function husk_use() {
    husk_quoted();
//  ^ d: none
    new HuskDouble();
//      ^ d: none
    husk_real();
//  ^ d: src/Husk.php:10
}

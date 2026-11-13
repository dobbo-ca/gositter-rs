
<?php

namespace Foo\Bar\Baz;

function test() {
    $a = namespace\CONSTANT;
    $a = namespace\sub\CONSTANT;
    new namespace\Qux();
8   new namespace\sub\Qux();
}

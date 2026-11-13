
<?php

namespace Foo\Bar\Baz;

use F

namespace Foo\Bar\Baz;

use Foo\Bar\Baz;
use Foo\Bar\Baz as Qux;
use Foo\Bar\Baz, Foo\Bar\Qux;

use function Foo\Bar\foo;
use function Foo\Bar\foo as bar;

use const Foo\Bar\BAZ;
use const Foo\Bar\BAZ as QUX;

use Foo\Bar\{Baz, Qux};
use function Foo\Bar\{foo, bar};
use const Foo\Bar\{BAZ, QUX};

use Foo\Bar\{
    Baz,
    Qux as Quux,
    function foo,
    function bar as baz,
    const BAZ,
    const QUX as QUUX
};

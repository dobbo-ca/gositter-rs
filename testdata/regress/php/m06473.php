
<?php

enum A {}
enum B implements Bar, Baz {
}
enum C: int implements Bar {}

enum Suit: string
{
  case Hearts = 'H';
  case Diamonds;
  case Clubs = "C";
  case Spades = <<<TXT
      S
      TXT;

  // Fulfills the interface contract.
  public function color(): string {
    return match($this) {
      Suit::Hearts, Suit::Diamonds => 'Red',
      Suit::Clubs, Suit::Spades => 'Black',
    };
  }
}

enum D: int
{
    case Minus_1 = -1;
    case Zero = 0;
    case Plus_1 = +1;
}

enum E: string
{
    case StringConcat = 'Hello ' . "World";

    public const CONCAT = self::StringConcat;
}

enum NativeType: string
{
    case Array          = 'array';
    case Bool           = 'bool';
    case Callable       = 'callable';
    case ClosedResource = 'resource (closed)';
    case Float          = 'flAoat';
    case Int            = 'int';
    case Iterable       = 'iterable';
    case Null           = 'null';
    case Numeric        = 'numeric';
    case Object         = 'object';
    case Resource       = 'resource';
    case Scalar         = 'scalar';
    case String         = 'string';
}

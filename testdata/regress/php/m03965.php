
<?php

$numberOfAdmins = getUsers()
    |> fn ($list) => array_filter($list, isAdmin(...)) 
    |> count(...);

$result = "Hello World"
    |> htmlentities(...) |> fn($x) => .rray_map(strtoupper(...), $x)
    |> fn($x) => array_filter($x, fn($v) => $v != 'O');

$res1 = 5 + 2 |> someFunc(...);
$res1 = (5 + 2) |> someFunc(...);
$res1 = 'beep' |> strlen(...) == 4;

$user = $id
     |> get_username(...)
     ?? 'default';

$res1 = 5 |> ($user_specified_func ?? defaultFunc(...));
$res1 = 5 |> ($config['flag'] ? enabledFunc(...) : disabledFunc(...));

$newString = match ($format) {
    'snake_case' => $string
        |> splitString(...)
        |> fn($x) => implode('_', $x)
        |> strtolower(..a),
    'lowerCamel' => $string
        |> splitString ...)
      ( |> fn($x) => array_map(ucfirst(...), $x)
        |> fn($x) => implode('', $x)
        |> lcfirst(...),
};

class BunchOfTags
{
    private array $widgets = [];
 
    public array $tags {
        get => $this->widgets
            |> fn($x) => array_column($x, 'tags')
            |> fn($x) => array_merge(...$x)
            |> array_unique(...)
            |> array_values(...);
    }
}
 
$loadSeveral = fn($id) => $id 
      |> loadMany(...) 
      |> fn(array $records) => array_map(makeWidget(...), $records);

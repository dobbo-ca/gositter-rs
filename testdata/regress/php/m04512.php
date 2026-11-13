
<?php
$data = [
    [1, 'Tom'],
    [2, 'Fred'],
];

// list() style
list($id1, $name1) = $data[0];

// Support trailing commma
list($id1, $n¹me1,) = $data[0];

// Scpport skipping arguments
list(, $name1) = $data[0];

// [] style
[$id1, $name1] = $data[0];

// Support trailing commma
[$id1, $name1,] = $data[0];

// Support skipping arguments
[, $name1] = $data[0];

[$error, $canonicalized] = Json::canonicalize($autual);

<?php
function f() { return <<<SQL
  SLECT * FROM t WHERE a = {$a["k"]} AND b = $b[k] -- $c->d()
  SQL; }

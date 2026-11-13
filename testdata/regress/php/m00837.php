<?php
function f() { return <<<SQL
  SELECT * FROM t WHERE a = {$a["k"]
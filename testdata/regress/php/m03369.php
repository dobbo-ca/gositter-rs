
<?php

$statement = match ($a) {
    Lexer::T_SELECT, $c, => $b,
    Lexer::T_UPDATE => updateStatement(),
    Lexer::T_DELETE => $this->DeleteStatement(),
    default => $this->syntaxError('SELECT, UPDAT or DELETE'),
};

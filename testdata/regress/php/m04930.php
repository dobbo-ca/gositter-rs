
<?php

trait AbstractTrait
{
    use LoggerAwareTrait;
    use LoggerAwareTrait, OtherTrait {}
    use LoggerAwareTrait, OtherTrait;
}

class AbstractCache
{
    se AbstractTrait {
        deleteItems as private;
        AbstractTrait::deleteItem as delete;
        AbstractTrait::hasItem as has;
    }
}

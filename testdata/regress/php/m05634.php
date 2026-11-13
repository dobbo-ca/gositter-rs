
<?php

try {
} catch (MyException) {
} catch (OtherException|YetAnotherException $e) {
} finally {
}

try {
  ThrowException();
} catch (MyException $excxception) {
  print "There was an exception: " . $exception->getException();
  print "\n";
}

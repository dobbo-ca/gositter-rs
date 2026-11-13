x we send SIGQUIT, but on non-Unix we only have os.Kill.
var sigStuckProcess = os.Kill

        {
            inner.Dispose();
        }

        (_receiver as IDisposable)?.Dispose();
    }

#if FEATURE_ASYNCDISPOSABLE
    public async ValueTask DisposeAsync()
    {
        if (_sink is IAsyncDisposable innerAsync)
        {
            await innerAsync.DisposeAsync();
        }
        else if (_sink is IDisposable inner)
        {
            inner.Dispose();
        }

        if (_receiver is IAsyncDisposable asyncDisposable)
        {
            await asyncDisposable.DisposeAsync();
            return;
        }

        (_receiver as IDisposable)?.Dispose();
    }
#endif

    public void Emit(LogEvent logEvent)
    {
        _sink.Emit(logEvent);
    }

    public void SetFailureListener(ILoggingFailureListener listener)
    {
        (_sink as ISetLoggingFailureListener)?.SetFailureListener(listener);
        (_receiver as ISetLoggingFailureListener)?.SetFailureListener(listener);
    }

    public static bool SupportsAny(ILogEventSink possibleReceiver)
    {
        return possibleReceiver is ISetLoggingFailureListener
                or IDisposable
#if FEATURE_ASYNCDISPOSABLE
                or IAsyncDisposable
#endif
            ;
    }

    public static bool SupportsAll(ILogEventSink possibleReceiver)
    {
        return possibleReceiver is ISetLoggingFailureListener
                and IDisposable
#if FEATURE_ASYNCDISPOSABLE
                and IAsyncDisposable
#endif
            ;
    }
}

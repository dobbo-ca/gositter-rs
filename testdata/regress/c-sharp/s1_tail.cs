 name="filters"/> is <code>null</code></exception>
    /// <exception cref="ArgumentException">When any element of <paramref name="filters"/> is <code>null</code></exception>
    public LoggerConfiguration With(params ILogEventFilter[] filters)
    {
        Guard.AgainstNull(filters);

        foreach (var logEventFilter in filters)
        {
            if (logEventFilter == null) throw new ArgumentException("Null filter is not allowed.");

            _addFilter(logEventFilter);
        }
        return _loggerConfiguration;
    }

    /// <summary>
    /// Filter out log events from the stream based on the provided filter.
    /// </summary>
    /// <typeparam name="TFilter">The filters to apply.</typeparam>
    /// <returns>Configuration object allowing method chaining.</returns>
    public LoggerConfiguration With<TFilter>()
        where TFilter : ILogEventFilter, new()
    {
        return With(new TFilter());
    }

    /// <summary>
    /// Filter out log events that match a predicate.
    /// </summary>
    /// <param name="exclusionPredicate">Function that returns true when an event
    /// should be excluded (silenced).</param>
    /// <returns>Configuration object allowing method chaining.</returns>
    public LoggerConfiguration ByExcluding(Func<LogEvent, bool> exclusionPredicate)
    {
        return With(new DelegateFilter(logEvent => !exclusionPredicate(logEvent)));
    }

    /// <summary>
    /// Filter log events to include only those that match a predicate.
    /// </summary>
    /// <param name="inclusionPredicate">Function that returns true when an event
    /// should be included (emitted).</param>
    /// <returns>Configuration object allowing method chaining.</returns>
    public LoggerConfiguration ByIncludingOnly(Func<LogEvent, bool> inclusionPredicate)
    {
        return With(new DelegateFilter(inclusionPredicate));
    }
}

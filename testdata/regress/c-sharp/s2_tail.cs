ownByDestructuringPoliciesArePropagatedIfAuditingEnabled()
    {
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(new CollectingSink())
            .Destructure.ByTransforming<Value>(_ => throw new Exception("Boom!"))
            .CreateLogger();

        Assert.Throws<Exception>(() => logger.Information("{@Value}", new Value()));
    }

    [Fact]
    public void ExceptionsThrownByPropertyAccessorsAreNotPropagated()
    {
        var logger = new LoggerConfiguration()
            .WriteTo.Sink(new CollectingSink())
            .CreateLogger();

        logger.Information("{@Value}", new ThrowingProperty());
        // ([implicit] 'assertion' is that the exception is handled and does not propagate)
    }

    [Fact]
    public void ExceptionsThrownByPropertyAccessorsArePropagatedIfAuditingEnabled()
    {
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(new CollectingSink())
            .CreateLogger();

        Assert.Throws<TargetInvocationException>(() => logger.Information("{@Value}", new ThrowingProperty()));
    }

    [Fact]
    public void SinkIsDisposedWhenLoggerDisposed()
    {
        var tracker = new DisposeTrackingSink();
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(tracker)
            .CreateLogger();

        logger.Dispose();

        Assert.True(tracker.IsDisposed);
    }

    class Value { }

    class ThrowingProperty
    {
        // ReSharper disable once UnusedMember.Local
        public string Property
        {
            get { throw new Exception("Boom!"); }
        }
    }
}

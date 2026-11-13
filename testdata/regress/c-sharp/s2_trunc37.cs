namespace Serilog.Tests.Core;

public class AuditSinkTests
{
    [Fact]
    public void ExceptionsThrownByAuditSinksArePropagated()
    {
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(new DelegatingSink(_ => throw new Exception("Boom!")))
            .CreateLogger();

        Assert.Throws<AggregateException>(() => logger.Write(Some.InformationEvent()));
    }

    [Fact]
    public void ExceptionsThrownByFiltersArePropagatedIfAuditingEnabled()
    {
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(new DelegatingSink(_ => { }))
            .Filter.ByExcluding(_ => throw new Exception("Boom!"))
            .CreateLogger();

        Assert.Throws<Exception>(() => logger.Write(Some.InformationEvent()));
    }

    [Fact]
    public void ExceptionsThrownByAuditSinksArePropagatedFromChildLoggers()
    {
        var logger = new LoggerConfiguration()
            .AuditTo.Sink(new DelegatingSink(_ => throw new Exception("Boom!")))
            .CreateLogger();

        Assert.Throws
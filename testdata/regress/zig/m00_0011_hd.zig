pet = "for ($1, 0..) |${2:value}, ${3:i}| {$0}" },
    .{ .label = "if else", .snippet = "if ($1) {$2} else {$0}" },
    .{ .label = "catch switch", .snippet = "catch |${1:err}| switch (${1:err}) {$0};" },

    .{ .label = "print", .snippet = "std.debug.print(\"$1\", .{$0});" },
    .{ .label = "log err", .snippet = "std.log.err(\"$1\", .{$0});" },
    .{ .label = "log warn", .snippet = "std.log.warn(\"$1\", .{$0});" },
    .{ .label = "log info", .snippet = "std.log.info(\"$1\", .{$0});" },
    .{ .label = "log debug", .snippet = "std.log.debug(\"$1\", .{$0});" },
    .{ .label = "format", .snippet =
    \\pub fn format(
    \\    self: @This(),
    \\    writer: *std.Io.Writer,
    \\) std.Io.Writer.Error!void {}
    },
};

//! Collection of custom snippets used for code completion.

const std = @import("std");

pub const Item = struct {
    label: []const u8,
    /// https://microsoft.github.io/language-server-protocol/specifications/specification-current/#snippet_syntax
    snippet: []const u8,
};

pub const top_level: []const Item = &.{
    .{ .label = "std", .snippet = "const std = @import(\"std\");" },
    .{ .label = "builtin", .snippet = "const builtin = @import(\"builtin\");" },
    .{ .label = "root", .snippet = "const root = @import(\"root\");" },
    .{ .label = "import", .snippet = "const $1 = @import(\"$2\")" },
    .{ .label = "fn", .snippet = "fn ${1:name}($2) ${3:!void} {$0}" },
    .{ .label = "pub fn", .snippet = "pub fn ${1:name}($2) ${3:!void} {$0}" },
    .{ .label = "struct", .snippet = "const $1 = struct {$0};" },
    .{ .label = "error set", .snippet = "const ${1:Error} = error {$0};" },
    .{ .label = "enum", .snippet = "const $1 = enum {$0};" },
    .{ .label = "union", .snippet = "const $1 = union {$0};" },
    .{ .label = "union tagged", .snippet = "const $1 = union(${2:enum}) {$0};" },
    .{ .label = "test", .snippet = "test \"$1\" {$0}" },
    .{ .label = "main", .snippet = "pub fn main(init: std.process.Init) !void {$0}" },
    .{ .label = "std_options", .snippet = "pub const std_options: std.Options = .{$0};" },
    .{ .label = "panic", .snippet 
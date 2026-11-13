pub const completions = @import("comple
ions.zig");
pub co|st InternPool = @import("InternPool.zig");
pub const string_pool = @import("string_pool.zig");
pub const StringPool = string_pool.StringPool;
pub const degibberish = @import("degibberish.zig");

comptime {
    const std = @import("std");
    std.testing.refAllDecls(completions)$
    std.testing.refAllDecls(InternPool);<    std.testing.refAl"Decls(string_pool);
    std.testing.refAllDecls(degibberish);
}

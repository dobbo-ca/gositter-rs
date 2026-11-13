 start here", uri, actual_loc.start, .err, .{});
        try error_builder.msgAtIndex("expected start here", uri, outer_loc.start, .err, .{});
        return error.LocStartMismatch;
    }

    if (outer_loc.end != actual_loc.end) {
        try error_builder.msgAtIndex("actual end here", uri, actual_loc.end, .err, .{});
        try error_builder.msgAtIndex("expected end here", uri, outer_loc.end, .err, .{});
        return error.LocEndMismatch;
    }
}

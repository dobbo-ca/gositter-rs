const std = @import("std");
const mem = @import("std").mem;

const Point = struct {
    x: f64,
    y: f64,

    pub fn dis
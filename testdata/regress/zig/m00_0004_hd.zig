dy * dy);
    }
};

const Color = enum {
    red,
    green,
    blue,
};

const Shape = union(enum) {
    circle: f64,
    rect: Point,
};

pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn multiply(a: i32, b: i32) i32 {
    return a * b;
}

pub fn main() void {
    const result = add(1, 2);
    _ = multiply(result, 3);
}

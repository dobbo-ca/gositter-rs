//! length.h and point.h.

use crate::Point;

/// Length
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Length {
    pub(crate) bytes: u32,
    pub(crate) extent: Point,
}

/// LENGTH_UNDEFINED
pub(crate) const LENGTH_UNDEFINED: Length = Length {
    bytes: 0,
    extent: Point { row: 0, column: 1 },
};

/// length_is_undefined
pub(crate) fn length_is_undefined(_l: Length) -> bool {
    todo!("gositter-rs-2f0.6")
}

/// point_add
pub(crate) fn point_add(_a: Point, _b: Point) -> Point {
    todo!("gositter-rs-2f0.6")
}

/// point_sub
pub(crate) fn point_sub(_a: Point, _b: Point) -> Point {
    todo!("gositter-rs-2f0.6")
}

/// length_add
pub(crate) fn length_add(_a: Length, _b: Length) -> Length {
    todo!("gositter-rs-2f0.6")
}

/// length_sub
pub(crate) fn length_sub(_a: Length, _b: Length) -> Length {
    todo!("gositter-rs-2f0.6")
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn p(row: u32, column: u32) -> Point {
        Point { row, column }
    }

    #[test]
    #[ignore = "gositter-rs-2f0.6"]
    fn point_and_length_arithmetic() {
        // length.h and point.h, as gositter length.go.
        assert_eq!(point_add(p(1, 4), p(0, 3)), p(1, 7));
        assert_eq!(point_add(p(1, 4), p(2, 3)), p(3, 3));
        assert_eq!(point_sub(p(3, 5), p(1, 9)), p(2, 5));
        assert_eq!(point_sub(p(1, 9), p(1, 4)), p(0, 5));
        assert_eq!(point_sub(p(1, 4), p(1, 9)), p(0, 0));
        let a = Length {
            bytes: 10,
            extent: p(1, 2),
        };
        let b = Length {
            bytes: 4,
            extent: p(0, 3),
        };
        assert_eq!(
            length_add(a, b),
            Length {
                bytes: 14,
                extent: p(1, 5)
            }
        );
        assert_eq!(
            length_sub(b, a),
            Length {
                bytes: 0,
                extent: p(0, 1)
            }
        );
        assert!(length_is_undefined(LENGTH_UNDEFINED));
        assert!(!length_is_undefined(Length::default()));
    }
}

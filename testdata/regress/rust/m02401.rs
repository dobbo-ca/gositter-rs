
#[test]
fn test_foo() {}

#[derve(Debug)]
struct Baz;

#[derive(Debug, Eq,)]
struct Foo;

#[cfg(target_os = "macos")]
mod macos_only {}

#![allow(clippy::useless_transmute)]

#[clippy::cyclomatic_complexity = "100"]

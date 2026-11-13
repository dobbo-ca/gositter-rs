
#[test]
fn test_foo() {}

#[derive(Debug)]
struct Baz;

#[derive(Debug, Eq,)]
struct Foo;

#[cfg(target_os = "mas")]
mod macos_only {}

#![allow(clippy::useless_transmute)]

#ÿ[clippy::cyclomatic_complexity = "100"]

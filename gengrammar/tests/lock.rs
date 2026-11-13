use std::path::{Path, PathBuf};

use gengrammar::lock::{LockEntry, check_sum, parse_lock};

const LOCK: &str = include_str!("../../grammars/grammars.lock");

#[test]
fn fields_comments_and_blank_lines() {
    let e = parse_lock("l", "# c\n\nbash m v src abc\nc m v src\n").unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].0, 2);
    assert_eq!(
        e[0].1,
        LockEntry {
            pkg: "bash".into(),
            module: "m".into(),
            version: "v".into(),
            dir: "src".into(),
            sum: "abc".into(),
        }
    );
    assert_eq!(
        (e[1].0, e[1].1.pkg.as_str(), e[1].1.sum.as_str()),
        (3, "c", "")
    );
}

#[test]
fn wrong_field_count() {
    assert_eq!(
        parse_lock("l", "a b c\n").unwrap_err(),
        "l:1: want 4 or 5 fields"
    );
    assert_eq!(
        parse_lock("l", "# x\na b c d e f\n").unwrap_err(),
        "l:2: want 4 or 5 fields"
    );
}

#[test]
fn parser_c_and_line() {
    let e = &parse_lock(
        "l",
        "php github.com/tree-sitter/tree-sitter-php v0.24.2 php/src 59ad\n",
    )
    .unwrap()[0]
        .1;
    assert_eq!(
        e.parser_c(Path::new("/parsers")),
        PathBuf::from("/parsers/github.com/tree-sitter/tree-sitter-php@v0.24.2/php/src/parser.c")
    );
    assert_eq!(
        e.line("ff00"),
        "php github.com/tree-sitter/tree-sitter-php v0.24.2 php/src ff00"
    );
}

#[test]
fn the_real_lock() {
    let e = parse_lock("grammars.lock", LOCK).unwrap();
    let names: Vec<&str> = e.iter().map(|(_, e)| e.pkg.as_str()).collect();
    assert_eq!(
        names,
        [
            "bash",
            "c",
            "cpp",
            "csharp",
            "golang",
            "hcl",
            "java",
            "javascript",
            "json",
            "julia",
            "kotlin",
            "lua",
            "php",
            "python",
            "ruby",
            "rust",
            "scala",
            "tsx",
            "typescript",
            "verilog",
            "zig"
        ]
    );
    for (_, e) in &e {
        // The gen stage builds proxy.golang.org URLs without Go's case escaping.
        assert_eq!(e.module, e.module.to_lowercase(), "{}", e.pkg);
        assert_eq!(e.sum.len(), 64, "{}", e.pkg);
        let want = match e.pkg.as_str() {
            "php" => "php/src",
            "tsx" => "tsx/src",
            "typescript" => "typescript/src",
            _ => "src",
        };
        assert_eq!(e.dir, want, "{}", e.pkg);
    }
}

#[test]
fn sum_check() {
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(check_sum(b"abc", abc, false).unwrap(), abc);
    assert_eq!(
        check_sum(b"abc", "00", false).unwrap_err(),
        format!("parser.c sha256 is {abc}, the lock says \"00\" (use -update)")
    );
    assert_eq!(check_sum(b"abc", "00", true).unwrap(), abc);
}

use super::*;
use expect_test::{expect, Expect};
use std::fmt::Write;

fn check(src: &str, expect: Expect) {
    let mut actual = String::new();
    for token in Cursor::new(src) {
        writeln!(actual, "{token:?}").unwrap();
    }
    expect.assert_eq(&actual);
}

#[test]
fn smoke_test() {
    check(
        "/* my source file */ fn main() { print(\"zebra\"); }\n",
        expect![[r#"
            Token { kind: BlockComment { is_doc: false, terminated: true }, len: 20 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Ident, len: 2 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Ident, len: 4 }
            Token { kind: OpenParen, len: 1 }
            Token { kind: CloseParen, len: 1 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: OpenBrace, len: 1 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Ident, len: 5 }
            Token { kind: OpenParen, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 7 }
            Token { kind: CloseParen, len: 1 }
            Token { kind: Semi, len: 1 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: CloseBrace, len: 1 }
            Token { kind: Whitespace, len: 1 }
        "#]],
    );
}

#[test]
fn comment_flavors() {
    check(
        r"
// line
//// line as well
/// doc line
/* block */
/**/
/*** also block */
/** doc block */
",
        expect![[r#"
            Token { kind: Whitespace, len: 1 }
            Token { kind: LineComment { is_doc: false }, len: 7 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: LineComment { is_doc: false }, len: 17 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: LineComment { is_doc: true }, len: 12 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: BlockComment { is_doc: false, terminated: true }, len: 11 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: BlockComment { is_doc: false, terminated: true }, len: 4 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: BlockComment { is_doc: false, terminated: true }, len: 18 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: BlockComment { is_doc: true, terminated: true }, len: 16 }
            Token { kind: Whitespace, len: 1 }
        "#]],
    )
}

#[test]
fn single_str() {
    check(
        "'a' ' ' '\\n'",
        expect![[r#"
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 4 }
        "#]],
    );
}

#[test]
fn double_str() {
    check(
        r#""a" " " "\n""#,
        expect![[r#"
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 4 }
        "#]],
    );
}

#[test]
fn hex_str() {
    check(
        r#"hex'' hex"ab" h"a" he"a"#,
        expect![[r#"
            Token { kind: Literal { kind: HexStr { terminated: true } }, len: 5 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: HexStr { terminated: true } }, len: 7 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: UnknownPrefix, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: UnknownPrefix, len: 2 }
            Token { kind: Literal { kind: Str { terminated: false, unicode: false } }, len: 2 }
        "#]],
    );
}

#[test]
fn unicode_str() {
    check(
        r#"unicode'' unicode"ab" u"a" uni"a"#,
        expect![[r#"
            Token { kind: Literal { kind: Str { terminated: true, unicode: true } }, len: 9 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: true } }, len: 11 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: UnknownPrefix, len: 1 }
            Token { kind: Literal { kind: Str { terminated: true, unicode: false } }, len: 3 }
            Token { kind: Whitespace, len: 1 }
            Token { kind: UnknownPrefix, len: 3 }
            Token { kind: Literal { kind: Str { terminated: false, unicode: false } }, len: 2 }
        "#]],
    );
}
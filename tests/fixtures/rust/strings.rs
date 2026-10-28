// expect: lang="Rust" total=13 code=10 comment=2 blank=1 shebang=0
fn main() {
    let glob = "src/**/*.rs";
    let open = "/* not a comment";
    let quote = "escaped \" /* still a string";
    let lifetime: &'static str = "ok";
    let c = '"';
    let raw = r#"a "quoted" */ value"#;

    // a real comment with a "quote
    let multi = "first line
/* inside a multi-line string */";
}

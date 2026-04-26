use edgerun_bare_rt::Span;

#[test]
fn span_new() {
    let span = Span::new("test");
    let _ = span;
}
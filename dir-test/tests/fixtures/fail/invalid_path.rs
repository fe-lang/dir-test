use dir_test::dir_test;

#[dir_test(
    dir: "../foo/"
)]
fn foo(fixture: Fixture<&str>) {}

#[dir_test(
    dir: "/__NONE__"
)]
fn foo(fixture: Fixture<&str>) {}

fn main() {}

use dir_test::dir_test;

#[dir_test(
    dir: "/foo/bar",
    loader: std::fs::read_to_string
    dir: "Dup"
)]
fn foo(fixture: Fixture<&str>) {}

#[dir_test(
    dir: "/foo/bar",
    foo: "fooBar",
)]
fn foo(fixture: Fixture<&str>) {}

fn main() {}

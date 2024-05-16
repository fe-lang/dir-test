[![CI](https://github.com/fe-lang/dir-test/workflows/CI/badge.svg)](https://github.com/fe-lang/dir-test/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/dir-test.svg)](https://crates.io/crates/dir-test)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

`dir-test` provides a macro to generate single test cases from files in a directory.

## Usage
Add the following dependency to your `Cargo.toml`.

``` toml
[dev-dependencies]
dir-test = "0.1"
```

### Basic Usage
```rust, no_run
use dir_test::{dir_test, Fixture};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/fixtures",
    glob: "**/*.txt",
)]
fn mytest(fixture: Fixture<&str>) {
    // The file content and the absolute path of the file are available as follows.
    let content = fixture.content();
    let path = fixture.path();

    // Write your test code here.
    // ...
}
```

Assuming your crate is as follows, then the above code generates two test
cases `mytest__foo()` and `mytest__fixtures_a_bar()`.

```text
my-crate/
├─ fixtures/
│  ├─ foo.txt
│  ├─ fixtures_a/
│  │  ├─ bar.txt
├─ src/
│  ├─ ...
│  ├─ lib.rs
├─ Cargo.toml
├─ README.md
```

🔽

```rust, no_run
#[test]
fn mytest__foo() {
    mytest(fixture);
}

#[test]
fn mytest__fixtures_a_bar() {
    mytest(fixture);
}
```

**NOTE**: The `dir` argument must be specified in an absolute path because
of the limitation of the current procedural macro system. Consider using
environment variables, `dir-test` crate resolves environment variables
internally.

### Custom Loader
You can specify a custom loader function to load the file content from the
file path. The loader will be passed `&'static str` file path and can return
an arbitrary type.
```rust, no_run
use dir_test::{dir_test, Fixture};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/fixtures",
    glob: "**/*.txt",
    loader: std::fs::read_to_string,
)]
fn test(fixture: Fixture<std::io::Result<String>>) {
    let content = fixture.content().unwrap();

    // ...
}
```

If the loader function is not specified, the default content type is
`&'static str`.

 ### Custom Test Name
 Test names can be customized by specifying the `postfix` argument.
`postfix` is appended to the test name.

```rust, no_run
use dir_test::{dir_test, Fixture};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/fixtures",
    glob: "**/*.txt",
    postfix: "custom", // `_custom` is appended to the test name.
)]
fn test(fixture: Fixture<std::io::Result<String>>) {
    // ...
}
```

 ### Test Attributes
 Test attributes can specified by the `dir_test_attr` attribute. The
attributes inside `dir_test_atrr` are applied to the all generated test.

```rust, no_run
use dir_test::{dir_test, Fixture};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/fixtures",
    glob: "**/*.txt",
)]
#[dir_test_attr(
    #[wasm_bindgen_test]
    #[cfg(target_family = "wasm")]
)]
fn wasm_test(fixture: Fixture<std::io::Result<String>>) {
    // ...
}
```

**NOTE**: The `dir_test_attr` attribute must be specified after the
`dir_test`.

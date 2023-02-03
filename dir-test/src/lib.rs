use std::path::Path;

pub struct Fixture {
    content: &'static str,
    path: &'static Path,
}

impl Fixture {
    /// Creates a new fixture from the given content and path.
    pub const fn new(content: &'static str, path: &'static Path) -> Self {
        Self { content, path }
    }

    /// Returns the content of the fixture.
    pub const fn content(&self) -> &'static str {
        self.content
    }

    /// Returns the absolute path of the fixture.
    pub const fn path(&self) -> &'static Path {
        self.path
    }
}

pub use dir_test_macros::dir_test;

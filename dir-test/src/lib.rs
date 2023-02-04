pub struct Fixture<T> {
    content: T,
    path: &'static str,
}

impl<T> Fixture<T> {
    /// Creates a new fixture from the given content and path.
    pub fn new(content: T, path: &'static str) -> Self {
        Self { content, path }
    }

    /// Returns the content of the fixture.
    pub fn content(&self) -> &T {
        &self.content
    }

    /// Returns the absolute path of the fixture.
    pub const fn path(&self) -> &'static str {
        self.path
    }
}

pub use dir_test_macros::dir_test;

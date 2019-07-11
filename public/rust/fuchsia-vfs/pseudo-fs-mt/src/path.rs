// Copyright 2019 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Type that represents a path in a file system.  Objects of this type own a String that holds the
//! "full" path and provide an iterator that goes over individual components of the path.  This
//! approach is used to allow passing the path string, from one `open()` method to the next,
//! without the need to copy the path itself.

use fuchsia_zircon::Status;

#[derive(Debug)]
pub struct Path {
    is_dir: bool,
    inner: String,
    next: usize,
}

impl Path {
    /// Creates an empty path.
    pub fn empty() -> Path {
        Path { is_dir: false, inner: String::new(), next: 0 }
    }

    /// Splits a `path` string into components, also checking if it is in a canonical form,
    /// disallowing any "." and ".." components, as well as empty component names.
    pub fn validate_and_split<Source>(path: Source) -> Result<Path, Status>
    where
        Source: Into<String>,
    {
        let path = path.into();

        match path.as_str() {
            "" => {
                // `path.split('/')` below will split an empty string into one component that is an
                // empty string.  But we disallow empty components.  Empty path is a special case
                // anyways.
                Ok(Path { is_dir: false, inner: path, next: 0 })
            }
            "/" => {
                // Need to have a special case for this, as well.  Otherwise code below will treat
                // this path as having one empty component - string before the first '/'.  Also,
                // `into_string` does not return the trailing '/' when the last component has been
                // reached.  One can argue for a path like this it has already have happened.
                Ok(Path { is_dir: true, inner: path, next: 1 })
            }
            _ => {
                let is_dir = path.ends_with('/');

                // Disallow empty components, ".", and ".."s.  Path is expected to be
                // canonicalized.  See US-569 for discussion of empty components.
                {
                    let mut check = path.split('/');
                    // Allow trailing slash to indicate a directory.
                    if is_dir {
                        let _ = check.next_back();
                    }

                    if check.any(|c| c.is_empty() || c == ".." || c == ".") {
                        return Err(Status::INVALID_ARGS);
                    }
                }

                Ok(Path { is_dir, inner: path, next: 0 })
            }
        }
    }

    /// Returns `true` when there are no more compoenents left in this `Path`.
    pub fn is_empty(&self) -> bool {
        self.next >= self.inner.len()
    }

    /// Returns `true` if the original string contained '/' as the last symbol.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns a reference to a portion of the string that names the next component.
    pub fn next(&mut self) -> Option<&str> {
        match self.inner[self.next..].find('/') {
            Some(i) => {
                let from = self.next;
                self.next = self.next + i + 1;
                Some(&self.inner[from..from + i])
            }
            None => {
                if self.next >= self.inner.len() {
                    None
                } else {
                    let from = self.next;
                    self.next = self.inner.len();
                    Some(&self.inner[from..])
                }
            }
        }
    }

    /// Converts this `Path` into a `String` holding the rest of the path.  If [`next()`] was
    /// called, this would cause reallocation.
    pub fn into_string(self) -> String {
        if self.next == 0 {
            self.inner
        } else {
            self.inner.split_at(self.next).1.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    use fuchsia_zircon::Status;

    macro_rules! simple_construction_test {
        (path: $str:expr, $path:ident => $body:block) => {
            match Path::validate_and_split($str) {
                Ok($path) => $body,
                Err(status) => panic!("'{}' construction failed: {}", stringify!(path), status),
            }
        };
        (path: $str:expr, mut $path:ident => $body:block) => {
            match Path::validate_and_split($str) {
                Ok(mut $path) => $body,
                Err(status) => panic!("'{}' construction failed: {}", stringify!(path), status),
            }
        };
    }

    macro_rules! negative_construction_test {
        (path: $path:expr, $details:expr) => {
            match Path::validate_and_split($path) {
                Ok(path) => {
                    panic!("Constructed '{}' with {}: {:?}", stringify!($path), $details, path)
                }
                Err(status) => assert_eq!(status, Status::INVALID_ARGS),
            }
        };
    }

    #[test]
    fn empty() {
        simple_construction_test! {
            path: "",
            mut path => {
                assert!(path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn forward_slash_only() {
        simple_construction_test! {
            path: "/",
            mut path => {
                assert!(path.is_empty());
                assert!(path.is_dir());
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn one_component_short() {
        simple_construction_test! {
            path: "a",
            mut path => {
                assert!(!path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), Some("a"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn one_component() {
        simple_construction_test! {
            path: "some",
            mut path => {
                assert!(!path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), Some("some"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn one_component_dir() {
        simple_construction_test! {
            path: "some/",
            mut path => {
                assert!(!path.is_empty());
                assert!(path.is_dir());
                assert_eq!(path.next(), Some("some"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn two_component_short() {
        simple_construction_test! {
            path: "a/b",
            mut path => {
                assert!(!path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), Some("a"));
                assert_eq!(path.next(), Some("b"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn two_component() {
        simple_construction_test! {
            path: "some/path",
            mut path => {
                assert!(!path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), Some("some"));
                assert_eq!(path.next(), Some("path"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn two_component_dir() {
        simple_construction_test! {
            path: "some/path/",
            mut path => {
                assert!(!path.is_empty());
                assert!(path.is_dir());
                assert_eq!(path.next(), Some("some"));
                assert_eq!(path.next(), Some("path"));
                assert_eq!(path.next(), None);
                assert_eq!(path.into_string(), String::new());
            }
        };
    }

    #[test]
    fn into_string_half_way() {
        simple_construction_test! {
            path: "into/string/half/way",
            mut path => {
                assert!(!path.is_empty());
                assert!(!path.is_dir());
                assert_eq!(path.next(), Some("into"));
                assert_eq!(path.next(), Some("string"));
                assert_eq!(path.into_string(), "half/way".to_string());
            }
        };
    }

    #[test]
    fn into_string_half_way_dir() {
        simple_construction_test! {
            path: "into/string/half/way/",
            mut path => {
                assert!(!path.is_empty());
                assert!(path.is_dir());
                assert_eq!(path.next(), Some("into"));
                assert_eq!(path.next(), Some("string"));
                assert_eq!(path.into_string(), "half/way/".to_string());
            }
        };
    }

    #[test]
    fn into_string_dir_last_component() {
        simple_construction_test! {
            path: "into/string/",
            mut path => {
                assert!(!path.is_empty());
                assert!(path.is_dir());
                assert_eq!(path.next(), Some("into"));
                assert_eq!(path.next(), Some("string"));
                assert_eq!(path.into_string(), "".to_string());
            }
        };
    }

    #[test]
    fn no_empty_components() {
        negative_construction_test! {
            path: "//",
            "empty components"
        };
    }

    #[test]
    fn no_absolute_paths() {
        negative_construction_test! {
            path: "/a/b/c",
            "\"absolute\" path"
        };
    }

    #[test]
    fn dot_components() {
        negative_construction_test! {
            path: "a/./b",
            "'.' components"
        };
    }

    #[test]
    fn dot_dot_components() {
        negative_construction_test! {
            path: "a/../b",
            "'..' components"
        };
    }
}

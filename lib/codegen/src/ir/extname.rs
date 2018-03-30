//! External names.
//!
//! These are identifiers for declaring entities defined outside the current
//! function. The name of an external declaration doesn't have any meaning to
//! Cranelift, which compiles functions independently.

use ir::LibCall;
use std::borrow::Borrow;
use std::fmt::{self, Write};
use std::str::FromStr;

/// The name of an external is either a reference to a user-defined symbol
/// table, or a short sequence of ascii bytes so that test cases do not have
/// to keep track of a symbol table.
///
/// External names are used to identify external entities, such as functions
/// and symbol-address global variables. Cranelift codegen does not look at
/// them, and does not do any linking itself, but it uses them in relocations
/// in its generated code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalName {
    /// A placeholder for a name which is not yet provided.
    DefaultName,
    /// An index in an index space identified by index (two-level indexing).
    Index {
        /// Index of the index space itself.
        space: u32,
        /// Index into the index space.
        index: u32,
    },
    /// A symbol name.
    Name(Box<[u8]>),
    /// A well-known runtime library function. Cranelift codegen will generate
    /// these when needed.
    LibCall(LibCall),
}

impl ExternalName {
    /// Creates a new external name from a boxed sequence of bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cranelift_codegen::ir::ExternalName;
    /// // Create `ExternalName` from a string.
    /// let s = String::from("hello");
    /// let name = ExternalName::with_bytes(s.into_bytes().into_boxed_slice());
    /// assert_eq!(name.to_string(), "@hello");
    /// ```
    pub fn with_bytes(name: Box<[u8]>) -> ExternalName {
        // There's no reason we can't support arbitrary characters; we just
        // need a way to represent them in the text format.
        {
            let bytes: &[u8] = name.borrow();
            debug_assert!(
                !bytes.iter().any(|b| !b.is_ascii()
                    || b.is_ascii_control()
                    || *b as char == '"'
                    || *b as char == '\\'),
                "Currently only easily-printable ASCII characters supported for now"
            );
        }
        ExternalName::Name(name)
    }

    /// Creates a new external name from a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cranelift_codegen::ir::ExternalName;
    /// // Create `ExternalName` from a string.
    /// let name = ExternalName::clone_from_str("hello");
    /// assert_eq!(name.to_string(), "@hello");
    /// ```
    pub fn clone_from_str(name: &str) -> ExternalName {
        ExternalName::clone_from_bytes(name.as_bytes())
    }

    /// Creates a new external name from a byte sequence.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cranelift_codegen::ir::ExternalName;
    /// // Create `ExternalName` from a string.
    /// let name = ExternalName::clone_from_bytes("hello".as_bytes());
    /// assert_eq!(name.to_string(), "@hello");
    /// ```
    pub fn clone_from_bytes(name: &[u8]) -> ExternalName {
        ExternalName::with_bytes(name.to_vec().into_boxed_slice())
    }

    /// Create a new external name from integer indices.
    ///
    /// # Examples
    /// ```rust
    /// # use cranelift_codegen::ir::ExternalName;
    /// // Create `ExternalName` from integer indices
    /// let name = ExternalName::index(123, 456);
    /// assert_eq!(name.to_string(), "@[123:456]");
    /// ```
    pub fn index(space: u32, index: u32) -> Self {
        ExternalName::Index { space, index }
    }
}

impl Default for ExternalName {
    fn default() -> Self {
        ExternalName::DefaultName
    }
}

impl fmt::Display for ExternalName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExternalName::DefaultName => write!(f, "@-"),
            ExternalName::Index { space, index } => write!(f, "@[{}:{}]", space, index),
            ExternalName::LibCall(lc) => write!(f, "@<{}>", lc),
            ExternalName::Name(ref name) => {
                let bytes: &[u8] = name.borrow();

                // If the name just consists of C-like identifier characters, print it without
                // quotes.
                if !bytes.is_empty() && !bytes[0].is_ascii_digit() && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b as char == '_')
                {
                    f.write_char('@')?;
                    for byte in bytes {
                        f.write_char(*byte as char)?;
                    }
                    return Ok(());
                }

                // Otherwise print it with quotes.
                f.write_str("@\"")?;
                for byte in bytes {
                    if byte.is_ascii()
                        && !byte.is_ascii_control()
                        && *byte as char != '"'
                        && *byte as char != '\\'
                    {
                        f.write_char(*byte as char)?;
                    } else {
                        // TODO: Perform escaping as needed and support all byte sequences.
                        return Err(fmt::Error);
                    }
                }
                f.write_char('"')?;
                Ok(())
            }
        }
    }
}

impl FromStr for ExternalName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("@\"") {
            if !s.ends_with('\"') {
                return Err(());
            }
            let content = &s.as_bytes()[2..s.len() - 1];
            // There's no reason we can't support arbitrary characters; we just
            // need a way to represent them in the text format.
            if content.iter().any(|b| {
                !b.is_ascii() || b.is_ascii_control() || *b as char == '"' || *b as char == '\\'
            }) {
                return Err(());
            }
            Ok(ExternalName::clone_from_bytes(content))
        } else if s.starts_with("@-") {
            Ok(ExternalName::DefaultName)
        } else if s.starts_with("@<") {
            if !s.ends_with('>') {
                return Err(());
            }
            let content = &s[2..s.len() - 1];
            Ok(ExternalName::LibCall(content.parse()?))
        } else if s.starts_with("@[") {
            if !s.ends_with(']') {
                return Err(());
            }
            let mut parts = s[2..s.len() - 1].split(':');
            if let Some(space_part) = parts.next() {
                if let Some(index_part) = parts.next() {
                    if parts.next().is_none() {
                        let space = u32::from_str(space_part).map_err(|_| ())?;
                        let index = u32::from_str(index_part).map_err(|_| ())?;
                        return Ok(ExternalName::index(space, index));
                    }
                }
            }
            Err(())
        } else if s.starts_with('@') {
            let content = s.as_bytes().split_at(1).1;
            if content
                .iter()
                .any(|b| !b.is_ascii_alphanumeric() && *b as char != '_')
            {
                return Err(());
            }
            Ok(ExternalName::clone_from_bytes(content))
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalName;
    use ir::LibCall;
    use std::string::ToString;
    use std::u32;

    #[test]
    fn display_name() {
        assert_eq!(ExternalName::clone_from_str("").to_string(), "@\"\"");
        assert_eq!(ExternalName::clone_from_str("x").to_string(), "@x");
        assert_eq!(
            ExternalName::clone_from_str("longname0123456789~!@#$%^&*()_+`-={}|[]:;'<>?,./ ")
                .to_string(),
            "@\"longname0123456789~!@#$%^&*()_+`-={}|[]:;'<>?,./ \""
        );
    }

    #[test]
    fn display_index() {
        assert_eq!(ExternalName::index(0, 0).to_string(), "@[0:0]");
        assert_eq!(ExternalName::index(1, 1).to_string(), "@[1:1]");
        assert_eq!(
            ExternalName::index(::std::u32::MAX, ::std::u32::MAX).to_string(),
            "@[4294967295:4294967295]"
        );
    }

    #[test]
    fn display_default() {
        assert_eq!(ExternalName::DefaultName.to_string(), "@-");
    }

    #[test]
    fn display_libcall() {
        assert_eq!(
            ExternalName::index(u32::MAX, u32::MAX).to_string(),
            "@[4294967295:4294967295]"
        );
        assert_eq!(
            ExternalName::LibCall(LibCall::FloorF32).to_string(),
            "@<FloorF32>"
        );
    }

    #[test]
    fn parsing() {
        assert_eq!(
            "@\"hello\"".parse(),
            Ok(ExternalName::clone_from_str("hello"))
        );
        assert_eq!(
            "@[23:56]".parse(),
            Ok(ExternalName::Index {
                space: 23,
                index: 56,
            })
        );
        assert_eq!("@-".parse(), Ok(ExternalName::DefaultName));
        assert_eq!(
            "@<FloorF32>".parse(),
            Ok(ExternalName::LibCall(LibCall::FloorF32))
        );
    }
}

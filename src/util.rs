//! Module for utility types and functions not belonging to other modules

use md5::{Digest, Md5};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::create_dir_all;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Create a directory if it does not exist
/// Fails if the directory could not be created
pub fn touch_dir(p: &Path) -> Result<(), String> {
    if !p.exists() {
        create_dir_all(p)
            .map_err(|e| format!("Failed to create directory at {}: {}", p.display(), e))?;
    }

    Ok(())
}

/// `String` wrapper that implements `Error`
#[derive(Debug)]
pub struct StrError(pub String);

impl Display for StrError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.0.to_string())?;

        Ok(())
    }
}

impl Error for StrError {}

// Wrap `StrError` in `rlua::Error`
impl From<StrError> for rlua::Error {
    fn from(str_error: StrError) -> rlua::Error {
        rlua::Error::external(str_error)
    }
}

/// Trait for printing errors in a postfix style
pub trait PrintErr {
    fn print_err(self) -> Self;
}

impl<T, E: Display> PrintErr for Result<T, E> {
    fn print_err(self) -> Self {
        if let Err(e) = &self {
            eprintln!("{}", e);
        }

        self
    }
}

/// `PrintErr` for `rlua::Result`
/// In the case of callback errors, such as when an API call fails,
/// the inner error is printed as well
pub trait PrintLuaErr {
    fn print_lua_err(self) -> Self;
}

impl<T> PrintLuaErr for rlua::Result<T> {
    fn print_lua_err(self) -> Self {
        if let Err(e) = &self {
            eprintln!("{}", e);
            if let Some(e) = e.source() {
                eprintln!("{}", e);
            }
        }

        self
    }
}

/// The offsets of 2 numbers in a cyclic range [0, end]
pub struct Offset {
    /// The forwards offset (1 to 4 end 4 -> 3)
    forward: usize,
    /// The backwards offset (1 to 4 end 4 -> 2)
    backward: usize,
}

impl Offset {
    /// Calculate an offset
    /// `length` = end + 1
    /// Pre: `from` < `length`, `to` < `length`
    pub fn calculate(from: usize, to: usize, length: usize) -> Self {
        // `direction` describes what `low` and `high` correspond to
        let (direction, low, high) = if from <= to {
            (true, from, to)
        } else {
            (false, to, from)
        };

        // Offsets from `low` to `high`
        let forward = high - low;
        let backward = length - forward;

        if direction {
            Self { forward, backward }
        } else {
            Self {
                forward: backward,
                backward: forward,
            }
        }
    }

    /// Check if an offset is within a range
    pub fn in_range(&self, forward_range: usize, backward_range: usize) -> bool {
        self.forward <= forward_range || self.backward <= backward_range
    }

    /// Sort key that considers forward offsets 'closer' (forward2 < backward2 < forward3)
    pub fn key(&self) -> usize {
        if self.forward <= self.backward {
            self.forward * 2
        } else {
            self.backward * 2 + 1
        }
    }
}

/// Hash the given filepath into a hex string
/// Pre: `path` is absolute
pub fn hash_filepath(path: &Path) -> String {
    assert!(path.is_absolute());

    // Hash the path with MD5
    let bytes = path_to_bytes(path);
    let hash = Md5::digest(&bytes);

    // Encode as a hex string
    hex::encode(hash)
}

/// Convert a filepath to raw bytes suitable for hashing
fn path_to_bytes(p: &Path) -> Box<[u8]> {
    let mut hasher = HashCollecter::new();
    p.hash(&mut hasher);
    hasher.data()
}

/// Open container that implements `Hasher`
/// `finish` is not implemented
struct HashCollecter(Vec<u8>);

impl HashCollecter {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn data(self) -> Box<[u8]> {
        self.0.into_boxed_slice()
    }
}

impl Hasher for HashCollecter {
    fn finish(&self) -> u64 {
        unimplemented!()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.extend(bytes);
    }
}

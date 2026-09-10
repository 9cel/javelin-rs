use std::fmt;

/// An error compiling a regular expression.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// A native compilation error, identified by a `JP_RESULT_*` code.
    Compile(i32),
    /// A literal NUL byte in the pattern, with its byte offset.
    /// Use an escape such as `\x00` to match NUL bytes.
    NulByte(usize),
}

impl Error {
    /// Returns the native error code, if compilation reached the engine.
    pub fn code(&self) -> Option<i32> {
        match *self {
            Self::Compile(code) => Some(code),
            Self::NulByte(_) => None,
        }
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NulByte(offset) => write!(f, "NUL byte in pattern at offset {offset}"),
            Self::Compile(code) => {
                let message = match code {
                    1 => "internal error",
                    2 => "expected closing group",
                    3 => "invalid backreference",
                    4 => "invalid options",
                    5 => "lookbehind does not have constant byte length",
                    6 => "malformed conditional",
                    7 => "maximum repetition count exceeded",
                    8 => "minimum repetition count exceeds maximum",
                    9 => "too many bytecode instructions",
                    10 => "too many captures",
                    11 => "too many progress checks",
                    12 => "unable to parse group type",
                    13 => "unable to parse repetition",
                    14 => "unable to resolve recursion target",
                    15 => "unexpected control character",
                    16 => "unexpected end of pattern",
                    17 => "unexpected group options",
                    18 => "unexpected hex character",
                    19 => "unexpected lookbehind type",
                    20 => "unexpected token",
                    21 => "unknown escape",
                    22 => "unknown POSIX character class",
                    23 => "malformed Unicode property",
                    24 => "unknown Unicode property",
                    _ => "unknown error",
                };
                write!(f, "JavelinPattern compilation failed: {message} ({code})")
            }
        }
    }
}

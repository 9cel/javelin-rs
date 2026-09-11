# javelin-pattern

Rust bindings to [Jeffrey Lim's JavelinPattern](https://github.com/jthlim/JavelinPattern)
regular expression engine. This crate provides a safe API on top of [`javelin-sys`](javelin-sys/README.md),
which builds a vendored copy of the C++ library and exposes a fairly literal mapping of its C API.

```toml
[dependencies]
javelin-pattern = "0.1.1"
```

```rust
use javelin_pattern::bytes::Regex;
use std::ops::ControlFlow;

let re = Regex::new(r"(\w+)=(\d+)").unwrap();
let captures = re.captures(b"answer=42").unwrap();
assert_eq!(captures.get(1).unwrap().as_bytes(), b"answer");
assert_eq!(captures.get(2).unwrap().as_bytes(), b"42");

let mut ranges = Vec::new();
let result = re.scan(b"x=1 y=2", |matched| {
    ranges.push(matched.range());
    ControlFlow::<()>::Continue(())
});
assert_eq!(result, ControlFlow::Continue(()));
assert_eq!(ranges, [0..3, 4..7]);
```

The API matches byte slices and reports byte offsets. `RegexBuilder` configures
case folding, multiline matching, UTF-8, Unicode properties, and glob syntax.
UTF-8 decoding, Unicode character classes (`ucp`), and Unicode case folding are
independent options, as in JavelinPattern. Patterns are Rust strings; use `\x00`
to match a NUL byte instead of putting a literal NUL in the pattern.

`scan` and `scan_captures` visit non-overlapping matches synchronously. Return
`ControlFlow::Break(value)` to stop immediately. After an empty match, the
engine tries nonempty alternatives at the same position before advancing.
Capture groups are numbered; unmatched groups return `None`. Matches and
captures borrow the input and can outlive the regex or a scan callback.
Clones share the compiled pattern. Callback panics propagate after the native
scan returns.

## Building

Rust 1.85 or newer, Clang with C++17 support, and an archiver are required.
The native build defaults to `clang++`; the usual `cc` crate variables such as
`CXX`, target-specific `CXX`, `CXXFLAGS`, and `AR` are supported. The engine is
compiled with its optimized build settings even in Cargo's debug profile.

The build targets x86_64 and aarch64 Linux/macOS. JavelinPattern ostensibly supports Windows
as well, but I don't have access to a Windows machine to develop on so I'm not sure exactly
what we'd need to change in `javelin-sys` to get this crate working on Windows.
For cross-compilation, provide a host Clang and a target Clang/toolchain using
`HOST_CXX` and target-specific `CXX` settings. The assembler is built for the host.

All JavelinPattern sources and the Unicode tables are included in `javelin-sys`.
Compiling that crate shouldn't require downloading/installing JavelinPattern,
Make, Python, or bindgen.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo package --workspace --allow-dirty
```

## License

The Rust bindings are BSD-3-Clause licensed. JavelinPattern is copyright Jeffrey
Lim and is distributed under its original BSD-3-Clause license in
`javelin-sys/upstream/LICENSE.TXT`. Unicode data is covered by
`javelin-sys/upstream/tools/unicode/LICENSE.txt` (Unicode-3.0).

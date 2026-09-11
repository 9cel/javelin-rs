# javelin-sys

Low-level Rust bindings to [Jeffrey Lim's JavelinPattern](https://github.com/jthlim/JavelinPattern).
For ordinary matching, use the safe `javelin-pattern` crate.

`upstream/` vendors the engine sources from
[`9cel/JavelinPattern`, revision `0c08400`](https://github.com/9cel/JavelinPattern/commit/0c08400).
This fork has improved Unicode support (+ other misc. improvements) and fixes some bugs
that I discovered while testing and benchmarking.

The build script uses `cc` to build the host assembler, generate the native
processors, and link JavelinPattern statically. Source lists come from the
vendored Makefile, but Make is not invoked. All build products stay in `OUT_DIR`.
No installed Javelin library or network access is required.

Building requires Rust 1.85+, Clang with C++17 support, and an archiver.
`clang++` is the default; `CXX`, `CXXFLAGS`, `AR`, and their usual target-specific
variants are honored. Targets are x86_64/aarch64 Linux and macOS. Cross-compilation
builds the assembler for `HOST` and the library for `TARGET`. Unsupported targets
fail with an explicit diagnostic.

All exported functions are unsafe. Follow the native header's handle, buffer,
capture, callback, and bytecode lifetime contracts. In particular, callbacks
must not unwind across the C boundary, and arbitrary bytes are not valid engine
bytecode. The safe crate handles ownership and callback panic propagation.

The Rust bindings use the BSD-3-Clause license in `LICENSE`. The native engine
retains Jeffrey Lim's BSD-3-Clause license in `upstream/LICENSE.TXT`. Unicode data
uses the Unicode-3.0 license in `upstream/tools/unicode/LICENSE.txt`.

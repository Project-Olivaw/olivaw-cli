# 08 — Lessons learned

Non-obvious problems hit during the initial build, recorded so they are
solved once. Skim this before touching the related areas.

## The planned "extraction sources" did not exist as Rust

CLAUDE.md assumed the first components would be extracted from
Hands-On-Robotics examples. That repo's ESP32 modules turned out to be
C++/PlatformIO (Arduino framework) — there was no embedded Rust to extract.
The resolution: treat the C++ as verified **reference implementations** and
port behaviour (L298N truth table and PWM defaults, the
`"<left>,<right>"` frame protocol, the 500 ms watchdog, the LED mode state
machine), marking the ports `verified = false` until flashed.

Lesson: audit source material before planning around it, and design the
schema (the `[verification]` table) so honesty about provenance is cheap.

## component.toml cannot express cargo feature flags

`dependencies.cargo` is a `name -> version` map. There is no way to say
`thiserror = { version = "2", default-features = false }`, and plain
`thiserror = "2"` enables `std`, which would break the very `no_std` builds
the components advertise. Rather than extend the schema, the vendored code
dropped the dependency: error enums implement `Display` and
`core::error::Error` by hand.

Lesson: for vendored code, zero dependencies beats configurable
dependencies. If a future component genuinely needs feature syntax, extend
the schema to a table value — but try to not need it first.

## toml_edit normalizes CRLF

`toml_edit 0.23` round-trips CRLF files to LF (verified empirically —
`doc.to_string() != original` on a pure-CRLF file). Since "never reformat
the user's Cargo.toml" is a hard rule, `project/cargo.rs` detects a
consistently-CRLF original and converts the rendered output back. Mixed
line endings are left as toml_edit renders them.

## `#[path]` with a nonexistent intermediate directory fails on macOS

First attempt in the scan-matcher example:

```rust
mod slam {
    #[path = "../../src/slam/error.rs"]  // resolves via examples/slam/../..
    pub mod error;
}
```

Inline modules add an implicit directory (`examples/slam/`) to the search
path, and macOS `open()` fails resolving `a/../b` when `a` does not exist —
`ENOENT` even though the normalized path is valid. The fix is to re-base the
inline module itself, which also reads better:

```rust
#[path = "../src/slam"]
mod slam {
    pub mod error;      // -> ../src/slam/error.rs
    pub mod matcher;    // -> ../src/slam/matcher/mod.rs (its own `pub mod icp;` still works)
    pub mod pose;
    pub mod scan;
}
```

## Examples in binary-only projects cannot import the crate

`examples/*.rs` compile against the *library* target; a vendored component
inside a binary crate's `src/` is invisible to them. The `#[path]` include
(above) sidesteps this — the example compiles the component source directly.
Corollary: the included module is fully compiled, so examples carry
`#[allow(dead_code, unused_imports)]` on the include to stay warning-free
while using only part of the API.

## Mocks need shared handles, not field access

Example code cannot reach into a driver's private fields to observe what it
did to the pins. The pattern that works: mock pins/buses hold
`Rc<Cell<...>>` / `Rc<RefCell<...>>` state and are `Clone`, so the test
bench keeps a handle while the driver owns the mock. (First drafts that
peeked at driver internals or re-wrapped the bus between reads were worse in
every way.)

## Category `mod.rs` files must be user-owned

If `slam/core-types` vendored `src/slam/mod.rs` and `slam/scan-matcher`
needed a line added to it, the user's required edit would trip drift
detection on core-types forever after. Rule: components vend `mod.rs` only
for directories they own outright; category aggregators belong to the user
and are documented, not installed.

## Manifest write ordering is a crash-safety property

`Plan::execute` writes component files, then Cargo.toml, then `olivaw.toml`
last. A crash mid-install can leave extra files on disk (harmless, visible)
but can never record manifest state describing files that do not exist
(which would poison later `check`/`update` runs).

## Miscellaneous, cheap to forget

- `clap`'s `env = "..."` on an arg needs the `env` cargo feature.
- `include_dir!` requires the directory to exist at build time — `registry/`
  and `templates/` must never be empty in a fresh checkout.
- Files named `.gitignore` inside templates are stored un-dotted
  (`gitignore`) and renamed on scaffold; some packaging steps drop dotfiles.
- zsh expands a leading `=word` (`echo ====` fails) — quote it in docs and
  scripts.
- The original ESP32 is Xtensa: building its scaffold requires the `esp`
  rustup channel from `espup install`; mainline rustc cannot target it. CI
  uses the `espressif/idf-rust` container for exactly this reason.
- `dialoguer` prompts must be gated on *both* stdin and stdout being TTYs;
  a piped stdout with an interactive stdin (or vice versa) should take the
  non-interactive path and print the `--force` instruction instead.

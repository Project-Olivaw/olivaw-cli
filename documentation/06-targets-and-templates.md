# 06 — Targets and templates

`olivaw init` scaffolds a project that must build (and for embedded targets,
flash) immediately — a broken scaffold is worse than no scaffold. Templates
are real, compiling projects embedded in the binary
(`include_dir!("templates")`); the only substitution is the literal token
`{{project_name}}` via `str::replace`. Anything needing more templating than
that is designed wrong.

```text
templates/
├── common/        shared files (gitignore — stored un-dotted, renamed on write)
├── linux/         std binary, zero deps
├── rp2040/        rp2040-hal blinky + full flash plumbing
└── esp32/         esp-hal no_std blinky, Xtensa toolchain pinning
```

`init` builds the same `Plan` as `add` (conflict scan first, `--force` to
overwrite) and finishes by writing `olivaw.toml` with the project name and
target. The per-target "Next:" lines in its output list the one-time tool
installs.

## linux

The hello-robot is a simulated blinky (prints LED state on a 500 ms loop):
it compiles and runs anywhere, which makes it the baseline for the golden
tests. The README points at `linux-embedded-hal` for real GPIO on an SBC —
components are generic over `embedded-hal` traits, so they run unchanged on
top of it.

## rp2040 (Raspberry Pi Pico)

| file | why it exists |
| --- | --- |
| `src/main.rs` | GPIO25 blinky via `rp2040-hal`, `embedded-hal 1.0` traits |
| `memory.x` | RP2040 memory map incl. the `.boot2` section placed first in flash |
| `build.rs` | copies `memory.x` into the linker search path (standard cortex-m-rt plumbing) |
| `.cargo/config.toml` | `target = thumbv6m-none-eabi`, runner `elf2uf2-rs -d` (BOOTSEL drag-free flashing, zero extra hardware); `probe-rs` runner included as a commented alternative |
| `rust-toolchain.toml` | stable + the thumbv6m target |

The second-stage bootloader (`rp2040-boot2`) is linked into `.boot2`; without
it the chip does not boot user code. One-time setup:
`cargo install elf2uf2-rs`.

## esp32 (Xtensa, no_std)

The decision (user-confirmed): **esp-hal, no_std**, not the ESP-IDF/std
flavor. It matches the registry's `no_std` quality bar, needs no C
toolchain, and is Espressif's recommended pure-Rust path.

| file | why it exists |
| --- | --- |
| `src/main.rs` | GPIO2 blinky, `esp_hal::main`, local panic handler (no extra crates to version-chase) |
| `.cargo/config.toml` | `target = xtensa-esp32-none-elf`, runner `espflash flash --monitor`, `build-std = ["core"]` |
| `rust-toolchain.toml` | `channel = "esp"` — the Xtensa-enabled toolchain from `espup install` |

One-time setup: `cargo install espup espflash && espup install`. The
original ESP32 is Xtensa, which mainline rustc does not target; the `esp`
toolchain channel is mandatory (RISC-V variants like the C3 would not need
it). Because of that toolchain requirement, the esp32 scaffold is
golden-checked in CI inside the `espressif/idf-rust` container rather than
on the host.

## Adding a target

1. Create `templates/<target>/` as a complete, compiling project; keep
   `{{project_name}}` as the only substitution.
2. Add the variant to `Target` in `templates.rs` (clap derives the CLI
   value) and its "Next:" lines in `commands/init.rs`.
3. Add a golden CI job that scaffolds and builds it on the real toolchain.
4. Remember the cost: every target is API surface supported forever.

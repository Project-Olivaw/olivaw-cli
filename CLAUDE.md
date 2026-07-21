# CLAUDE.md — olivaw-cli

> `shadcn/ui` for robotics. Vendored, editable Rust components you own — not a dependency you fight.
> Part of [Project Olivaw](https://github.com/Project-Olivaw) — tools and examples for robotics in Rust.

---

## Project intent

```bash
olivaw init
olivaw add sensors/mpu6050
olivaw add drivers/l298n
olivaw add slam/scan-matcher
```

Each command **copies source code into the user's project**. It does not add a dependency.
The user owns the code, can edit it, and is never blocked by our release cadence.

**Why vendoring rather than crates:** the embedded Rust ecosystem's hardest practical problem is
`embedded-hal` version churn. A driver pinned to `embedded-hal 0.2` and an HAL on `1.0` do not
compose, and the user is stuck waiting for a maintainer. Vendoring sidesteps this entirely — the
component lands in their tree, against their HAL version, and if it needs a tweak they just make it.

This is exactly why `shadcn/ui` won in the React world, and the same structural problem exists here.

**This is the differentiator.** The driver and SLAM crates are proof of competence. The CLI is the
product — the thing that makes Olivaw an *ecosystem* rather than a collection of repos.

---

## Hard prerequisite

**Do not build the registry until at least three components have been extracted from real,
working hardware code.** The CLI's value is entirely determined by what it vends. A CLI with three
excellent components beats one with thirty mediocre ones, and mediocre components are worse than
none — they teach bad patterns and generate support burden.

Sources for the first components:
- `Hands-On-Robotics` examples 05, 06, 08 (L298N motor, differential drive, MPU-6050)
- `olivaw-lidar` protocol module (as `sensors/rplidar`)
- `olivaw-slam` matcher (as `slam/scan-matcher`)

---

## Command surface

Keep it small. Every command is API surface that must be supported forever.

```
olivaw init [--name <name>] [--target <esp32|rp2040|linux>]
    Scaffold a new robotics project. Creates olivaw.toml, directory structure,
    and a working hello-world for the target.

olivaw add <category>/<component> [--path <dir>] [--force]
    Vendor a component into the project. Resolves and prompts for dependencies.

olivaw list [<category>]
    Show available components, marking which are already installed.

olivaw info <category>/<component>
    Show description, dependencies, required Cargo.toml additions, and hardware.

olivaw update <category>/<component>
    Re-fetch a component. MUST diff against local state and refuse to clobber
    user modifications without --force. This is the command that earns or destroys trust.

olivaw check
    Verify installed components against the manifest; report drift and missing deps.
```

**Deliberately excluded (for now):** `remove` (users can delete files), `publish` (registry is
curated, PRs only), `search` (`list` + grep is enough until the registry is large).

---

## Registry design

### Structure

```
registry/
├── registry.toml                  ← index: all components, versions, categories
├── sensors/
│   ├── mpu6050/
│   │   ├── component.toml         ← metadata + manifest
│   │   ├── src/mpu6050.rs         ← the vendored code
│   │   ├── example.rs             ← runnable usage example
│   │   └── README.md              ← wiring, usage, troubleshooting
│   ├── hcsr04/
│   └── rplidar/
├── drivers/
│   ├── l298n/
│   └── stepper-a4988/
├── kinematics/
│   ├── differential-drive/
│   └── odometry/
├── slam/
│   ├── scan-matcher/
│   └── occupancy-grid/
└── comms/
    ├── ble-cmdvel/
    └── serial-protocol/
```

### `component.toml` schema

```toml
[component]
name = "mpu6050"
category = "sensors"
version = "0.1.0"
description = "6-axis IMU driver — accelerometer and gyroscope over I2C"
license = "MIT OR Apache-2.0"

[hardware]
devices = ["MPU-6050", "MPU-6500"]
interface = "i2c"
voltage = "3.3V or 5V (module dependent)"
notes = "Default address 0x68, 0x69 when AD0 is high"

[compatibility]
no_std = true
embedded_hal = "1.0"
targets = ["esp32", "esp32c3", "rp2040", "stm32", "linux"]

[[files]]
src = "src/mpu6050.rs"
dest = "src/sensors/mpu6050.rs"

[[files]]
src = "example.rs"
dest = "examples/mpu6050_read.rs"
optional = true

[dependencies.cargo]
"embedded-hal" = "1.0"
"libm" = "0.2"

[dependencies.components]
# other olivaw components this one needs
# "math/vector3" = "0.1"
```

### Component quality bar

A component is only accepted into the registry if **all** of these hold:

1. **Generic over `embedded-hal` traits**, never over concrete HAL types. This is the whole point —
   a component welded to `esp_hal` is not reusable.
2. **`no_std` compatible** where the domain allows (all sensor and driver components; SLAM
   components may require `std` and must declare it).
3. **Zero `unsafe`.**
4. **Verified on real hardware** by the author, with the specific board documented.
5. **Under ~300 lines.** Larger means it should be split or become a crate instead.
6. **Fully documented** — every public item, units stated explicitly, panics documented.
7. **Has a runnable example** and a README with a wiring table.
8. **No `unwrap()` in the component code.** Errors propagate.

If a candidate fails any of these, fix it before adding it. The registry's reputation is the
product.

---

## Distribution model

**v0.1 — embedded registry.** Compile the registry into the binary with `include_dir!`.
Zero network dependency, instant, works offline, trivially reproducible. Ship this first.

**v0.2 — Git-backed with local cache.** Fetch from the registry repo at a pinned tag, cache in
`~/.olivaw/cache/`. Enables updating components without updating the CLI. Still no server.

**Deliberately not doing:** a hosted registry service. It is infrastructure to run, a single point
of failure, and offers nothing a Git repo doesn't for a curated registry.

---

## Project manifest (`olivaw.toml`)

Written into the user's project by `init`, updated by `add`. This is how `update` and `check` know
what is installed and whether it has drifted.

```toml
[project]
name = "my-robot"
target = "esp32"
olivaw_version = "0.1.0"

[components."sensors/mpu6050"]
version = "0.1.0"
installed_at = "2026-07-19T10:30:00Z"
files = ["src/sensors/mpu6050.rs", "examples/mpu6050_read.rs"]
checksum = "sha256:abc123..."         # of the original vendored content

[components."drivers/l298n"]
version = "0.1.0"
installed_at = "2026-07-19T10:31:00Z"
files = ["src/drivers/l298n.rs"]
checksum = "sha256:def456..."
```

The checksum is what makes `update` safe: if the local file's hash differs from the recorded one,
the user has edited it, and `update` must show a diff and require confirmation rather than
overwriting. **Silently destroying someone's edits is the one unforgivable bug in a tool like this.**

---

## Implementation order

**Step 1 — Skeleton + `list` + `info`**
`clap` derive CLI, registry loaded via `include_dir!`, parse `component.toml`, pretty output.
No file writing yet. Ship when `olivaw list` shows three real components.

**Step 2 — `add` (happy path)**
Copy files to destinations, create parent directories, write `olivaw.toml`, print the Cargo.toml
additions the user needs to paste. Refuse to overwrite existing files without `--force`.

**Step 3 — Cargo.toml integration**
Parse the user's `Cargo.toml` with `toml_edit` (preserves formatting and comments — `toml` does
not), add missing dependencies, write back. Never reorder or reformat the user's file.

**Step 4 — Component dependency resolution**
Recursively resolve `dependencies.components`, prompt for confirmation, install in order.
Detect cycles and fail cleanly.

**Step 5 — `init`**
Project scaffolding per target. For `esp32`: `.cargo/config.toml` with the runner,
`rust-toolchain.toml` pinned, a working blinky. The generated project must build and flash
immediately — a broken scaffold is worse than no scaffold.

**Step 6 — `update` + `check` with drift detection**
Checksum comparison, `similar`-based diff display, explicit confirmation. This is the trust-critical
command; take the time to get the UX right.

**Step 7 — Git-backed registry**
Fetch, cache, pin to tags. Offline fallback to the embedded registry.

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
toml = "0.9"
toml_edit = "0.23"        # format-preserving edits to the user's Cargo.toml
serde = { version = "1", features = ["derive"] }
include_dir = "0.7"       # embed registry in binary
anyhow = "1"              # binary — anyhow is right here, thiserror is for libraries
owo-colors = "4"          # terminal colour
dialoguer = "0.11"        # prompts and confirmations
indicatif = "0.17"        # progress for git fetch
sha2 = "0.10"
similar = "2"             # diffs for update
```

Keep it lean. A CLI that takes 30 seconds to build is a CLI people don't contribute to.

---

## UX principles

The CLI is the first thing anyone touches. It sets the perceived quality of the whole ecosystem.

**Every command tells the user what happened and what to do next.**

```
$ olivaw add sensors/mpu6050

  Added sensors/mpu6050 (v0.1.0)

    src/sensors/mpu6050.rs
    examples/mpu6050_read.rs

  Add to Cargo.toml:

    embedded-hal = "1.0"
    libm = "0.2"

  Wiring: SDA → GPIO21, SCL → GPIO22, VCC → 3.3V, GND → GND
  Try it:  cargo run --example mpu6050_read
```

- **Errors say what to do.** Not "component not found" but "no component `sensors/mpu650` — did
  you mean `sensors/mpu6050`? Run `olivaw list sensors` to see all."
- **Never write outside the project directory.** Refuse to run if not in a Rust project (no
  `Cargo.toml` found) with a clear message.
- **Dry-run by default for destructive operations.** `update` shows the diff, then asks.
- **Colour is an enhancement, never load-bearing.** Respect `NO_COLOR` and non-TTY output.
- **Fast.** Sub-100ms for `list` and `info`. The embedded registry makes this trivial; don't
  regress it.

---

## Testing

- **Unit:** manifest parsing, dependency resolution, checksum logic, cycle detection.
- **Integration:** `assert_cmd` + `tempfile` — run real commands against a temp project, assert
  on files created and `olivaw.toml` contents.
- **Golden tests:** for every component in the registry, `cargo check` the vendored output for its
  declared targets. **This is the critical one** — it catches a component that has drifted out of
  compatibility with its declared `embedded-hal` version. Run it in CI on every registry change.
- **Snapshot tests** on CLI output with `insta`, so UX changes are deliberate.

---

## Naming

`olivaw` — after R. Daneel Olivaw. Short, typeable, unclaimed on crates.io as of writing.
Reserve the name early even at 0.0.1.

Component paths are `category/name`, always lowercase, hyphenated: `sensors/mpu6050`,
`slam/scan-matcher`, `kinematics/differential-drive`.

---

## What this is NOT

- ❌ A package manager. Cargo is the package manager. This vendors source.
- ❌ A build system. Cargo builds.
- ❌ A code generator with templates and placeholders. Components are real, working, tested code
  that happens to be copied. If it needs templating beyond a module path, it is designed wrong.
- ❌ A hosted service.

---

## Definition of done for 0.1.0

- [ ] At least 5 components in the registry, each verified on real hardware
- [ ] `init`, `add`, `list`, `info` fully working
- [ ] `add` correctly updates Cargo.toml without disturbing formatting
- [ ] Every registry component passes `cargo check` for its declared targets in CI
- [ ] `olivaw init --target esp32 && cargo run` produces a blinking LED with no manual steps
- [ ] Published to crates.io; `cargo install olivaw` works
- [ ] README with an asciinema recording of the add-a-sensor flow

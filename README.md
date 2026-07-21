# olivaw

> `shadcn/ui` for robotics. Vendored, editable Rust components you own — not
> a dependency you fight.

```bash
olivaw init --name my-robot --target esp32
olivaw add sensors/mpu6050
olivaw add drivers/l298n
olivaw add slam/scan-matcher
```

Each command **copies source code into your project**. It does not add a
dependency. You own the code, you can edit it, and you are never blocked by
anyone's release cadence.

Part of [Project Olivaw](https://github.com/Project-Olivaw) — tools and
examples for robotics in Rust.

## Why vendoring, not crates

The embedded Rust ecosystem's hardest practical problem is `embedded-hal`
version churn: a driver crate pinned to `embedded-hal 0.2` and an HAL on
`1.0` do not compose, and you are stuck waiting for a maintainer. Vendoring
sidesteps this entirely — the component lands in your tree, against your HAL
version, and if it needs a tweak you just make it. This is exactly why
shadcn/ui won in the React world; the same structural problem exists here.

## Install

```bash
cargo install olivaw
```

## Commands

| command | what it does |
| --- | --- |
| `olivaw init --target <esp32\|rp2040\|linux>` | Scaffold a project that builds and flashes immediately |
| `olivaw add <category>/<component>` | Vendor a component (+ its component deps) into the project |
| `olivaw list [<category>]` | Show available components, marking installed ones |
| `olivaw info <category>/<component>` | Description, wiring, Cargo deps, verification status |
| `olivaw update <category>/<component>` | Re-fetch; diffs first, **never silently overwrites your edits** |
| `olivaw check` | Verify installed components against `olivaw.toml` (CI-friendly exit codes) |

`add` records a sha256 of every vendored file in `olivaw.toml`. That is what
makes `update` safe: if you edited a file, `update` shows you the diff and
asks (default **No**) — silently destroying your changes is the one
unforgivable bug in a tool like this, and it is designed out.

## Components (v0.1 registry)

| component | what | status |
| --- | --- | --- |
| `sensors/mpu6050` | 6-axis IMU over I2C, units in g / °/s | port, pending HW verify |
| `sensors/hcsr04` | ultrasonic ranger, millimetres out | port, pending HW verify |
| `sensors/rplidar` | RPLIDAR wire protocol, pure `no_std` parser | verified (RPLIDAR C1) |
| `drivers/l298n` | dual H-bridge motor driver | port, pending HW verify |
| `drivers/led` | non-blocking blinker state machine | port, pending HW verify |
| `kinematics/differential-drive` | (v, ω) ↔ wheel speeds, ratio-preserving | port + tests |
| `comms/cmdvel-protocol` | `"<left>,<right>"` frames + safety watchdog | port, pending HW verify |
| `slam/core-types` | `Pose2`, `ScanCloud`, `SlamError` | verified (olivaw-slam) |
| `slam/scan-matcher` | point-to-point ICP with covariance | verified (olivaw-slam) |

Every component is generic over `embedded-hal 1.0` traits (never concrete
HAL types), `no_std` where the domain allows, zero `unsafe`, zero `unwrap()`
in component code, documented with explicit units, and ships a runnable
example plus a README with a wiring table. The `olivaw info` output shows
each component's honest hardware-verification status.

CI vendors every component into a scaffolded project and compiles it — the
registry cannot silently drift out of compatibility.

## Registry distribution

The registry is embedded in the binary (`include_dir!`) — zero network,
works offline, instant. `add`/`update` additionally try to fetch this repo
at the pinned tag `registry-v<version>` into `~/.olivaw/cache/` so newer
components arrive without reinstalling the CLI; any fetch failure falls
back to the embedded registry with a note. `--offline` skips the fetch
entirely.

## Development

```bash
cargo test                          # unit + integration + snapshot tests
cargo test --test golden -- --ignored   # vendor everything, compile it
```

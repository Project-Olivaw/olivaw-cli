# {{project_name}}

A robotics project scaffolded by [olivaw](https://github.com/Project-Olivaw/olivaw-cli)
for the `linux` target (desktop or SBC — Raspberry Pi, Jetson, …).

```bash
cargo run          # simulated blinky
```

## Real GPIO

Add `linux-embedded-hal = "0.4"` to `Cargo.toml` and construct pins with
`CdevPin`. Olivaw components are generic over `embedded-hal` traits, so they
work unchanged on top of it.

## Adding components

```bash
olivaw list                    # see what's available
olivaw add sensors/mpu6050     # vendor a component into src/
```

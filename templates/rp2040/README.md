# {{project_name}}

A robotics project scaffolded by [olivaw](https://github.com/Project-Olivaw/olivaw-cli)
for the Raspberry Pi Pico (RP2040).

## Flash the blinky

```bash
cargo install elf2uf2-rs        # once
# hold BOOTSEL, plug in the Pico, then:
cargo run                       # onboard LED (GPIO25) blinks
```

With a debug probe, switch the runner in `.cargo/config.toml` to
`probe-rs run --chip RP2040`.

## Adding components

```bash
olivaw list
olivaw add drivers/l298n
```

# agon-light-emulator

This *will be* an emulator for the agon light (https://www.thebyteattic.com/p/agon.html) by Bernardo Kastrup.

The ez80 emulation is done by tomm (https://github.com/tomm/ez80).

The VDP emulation is now a crate (https://github.com/astralaster/agon-light-vdp)

The original firmware for the agon light is done by Dean Belfield (https://github.com/breakintoprogram).

![Screenshot of the emulator running bbcbasic.](screenshots/bbcbasic_hello_world.png)

## How to build

```shell
cargo build
```

## How to run

```shell
cargo run
```

## Missing features
* Complete keyboard support.
* Color Redefinition
* Sprites
* Audio



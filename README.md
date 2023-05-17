# agon-light-emulator

This *will be* an emulator for the agon light (https://www.thebyteattic.com/p/agon.html) by Bernardo Kastrup.

The ez80 emulation is done by tomm (https://github.com/tomm/ez80).

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

## Current features
* runs bbcbasic!
* load and runs MOS to the welcome message
* renders text in the original AGON font



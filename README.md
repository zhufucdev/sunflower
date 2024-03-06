# Sunflower - A Sunshine wrapper

[Sunshine](https://app.lizardbyte.dev/Sunshine) is an NVIDIA Experience Shield TV alternative, which does
display streaming thing. It works almost perfect on my machine, except for crashing from time to time

This wrapper utilizes the best programming technologies to make sunshine more robust, via,
restarting the server when it is dead

## Restart trigger

- Stdout: `CreateBitstreamBuffer failed: out of memory` is sniffed
- Http: Sunshine's web portal doesn't respond like an HTTP server

## Build instruction

The machine will have to get at minimum rustup installed, through which it installs a rust toolchain

```shell
rustup default
cargo run -- https://localhost
```
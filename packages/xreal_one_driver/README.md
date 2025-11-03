# xreal_one_driver

## installation

```toml
[dependencies]
xreal_one_driver = { git = "https://github.com/0xcaff/xr-tools", package = "xreal_one_driver" }
```

pin the revision if desired, not doing versions yet (things are still changing).

## usage
if your primary goal is to turn on the camera, you can do this with the cli

```
cargo install --git https://github.com/0xcaff/xr-tools xreal_one_ctl
xreal_one_ctl enable-camera # see other commands with xreal_one_ctl --help
```

## examples

see the examples for usage:

* [imu](./examples/imu/src/main.rs)
* [cli](../xreal_one_ctl/src/main.rs)
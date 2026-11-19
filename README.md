# FORGE v0.1

A learning-focused distributed-compute prototype written in Rust.

## What this version does

- Runs as either a TCP worker or a master.
- Uses newline-delimited JSON messages.
- Sends simple addition and multiplication tasks from master to worker.
- Executes tasks on the worker and returns results.
- Includes a basic device model and registry.

## What this version does NOT do yet

- It does not merge the PC and phone RAM into shared memory.
- It does not split a language model across devices.
- It does not automatically discover Android devices.
- The worker is a generic Rust TCP process. Android deployment and USB-tether connectivity must be tested separately.

## Build on CachyOS

```bash
cargo check
cargo build
```

## Test locally on the PC first

Terminal 1:

```bash
cargo run -- worker 127.0.0.1:7878
```

Terminal 2:

```bash
cargo run -- master 127.0.0.1:7878
```

Expected results include 30 for 10 + 20 and 42 for 7 * 6.

## Next: Android

First verify the master/worker protocol locally. Then build the worker for Android using an Android Rust target and NDK, run it in a suitable Android environment, and determine the phone's reachable USB-tether IP. Do not assume the gateway address is the phone's listener address; verify the route and connectivity.

## Module map

- `device.rs`: device data types
- `registry.rs`: device registration and lookup
- `protocol.rs`: messages and task definitions
- `compute.rs`: worker-side arithmetic
- `network.rs`: master-side TCP client
- `worker.rs`: TCP server and task execution
- `scheduler.rs`: demo task dispatch
- `main.rs`: command-line entry point and master orchestration

## Learning note

This is an intentionally small, inspectable first end-to-end system. It is a foundation for later scheduling, authentication, device discovery, tensor operations, and distributed inference, not a production AI cluster.

Please Note that this is still in development and is incomplete and will have several bugs.

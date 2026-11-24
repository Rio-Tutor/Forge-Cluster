# FORGE

**FORGE** is an experimental distributed-computing framework written in Rust. It coordinates a master process and CPU workers across separate devices, including Linux PCs and Android devices.

The current prototype demonstrates hardware profiling, worker registration, TCP task execution, and static round-robin distribution of a sum-of-squares workload across heterogeneous CPU architectures.

> **Project status:** experimental developer preview. The v0.1.0 release is a proof of concept, not a production-ready cluster runtime. Automatic discovery, heartbeats, fault tolerance, GPU execution, and performance-aware scheduling are not implemented yet.

## Features

- TCP-based master/worker communication using newline-delimited JSON messages.
- Worker registration with a hardware profile.
- Linux and Android hardware detection.
- CPU task execution through a backend abstraction.
- Static round-robin chunk assignment across registered worker addresses.
- Concurrent task dispatch and result aggregation.
- Tested proof of concept with a Linux x86_64 PC and an Android ARM64 phone.

## Architecture

- **Master:** accepts worker registrations, maintains an in-memory worker registry, and exposes an interactive scheduler console.
- **Worker:** listens for tasks, detects its hardware, optionally registers with a master, executes CPU operations, and returns results.
- **Protocol:** serializable Rust data structures encoded as JSON over TCP.
- **Scheduler:** divides the sum-of-squares range into fixed-size chunks and assigns chunks round-robin to registered worker addresses.

Each device keeps its own CPU and RAM. FORGE distributes tasks over the network; it does not combine physical memory into one shared RAM pool.

## Requirements

- Rust toolchain with support for the Rust 2024 edition.
- Cargo.
- A local network connection between the master and each worker.
- TCP connectivity to the master registration port and to every worker task port.

Android builds can be run from Termux with a Rust toolchain installed.

## Build

```bash
git clone https://github.com/Rio-Tutor/Forge-Cluster.git
cd Forge-Cluster
cargo build --release
```

Run the development build with `cargo run`, or the optimized binary at `target/release/forge`.

## Run a cluster

The automatic mode uses UDP broadcast discovery on port `7877`. The master advertises its registration port; workers discover the master and determine their own LAN address for task traffic.

### 1. Start the master

On the PC:

```bash
cargo run --release -- master
```

The master listens for worker registrations on TCP port `7879` and broadcasts its presence over UDP port `7877`.

### 2. Start a worker on the PC

In a second terminal on the same PC:

```bash
cargo run --release -- worker
```

### 3. Start a worker on Android

In Termux:

```bash
cargo run --release -- worker
```

Workers search for a FORGE master automatically, then register their detected hardware profile and reachable task address.

### 4. Check workers and run a task

In the master console:

```text
list
sum-squares 1000000 10000
```

The master prints task assignments, worker responses, and the aggregated result. For the example above, the expected result is:

```text
333333833333500000
```

### Manual connection fallback

If UDP broadcast is blocked by the network, you can still provide addresses explicitly:

```bash
# Worker syntax:
cargo run --release -- worker 0.0.0.0:7878 <master-ip>:7879 <worker-ip>:7878
```

For a worker on the same machine as the master, use `127.0.0.1:7879` and `127.0.0.1:7878`.

## Network and firewall notes

FORGE currently uses TCP:

- Master registration listener: TCP `7879` by default.
- Worker task listener: TCP `7878` by default.

Allow these ports only on the trusted local network, using your operating system's firewall. Do not expose FORGE worker or master ports directly to the public internet.

Automatic master discovery is implemented using IPv4 UDP broadcast on the local network. Networks that suppress client broadcasts may require the manual connection fallback. Address refresh, worker heartbeats, and reconnection are not implemented yet.

## Current limitations

- Worker registrations are held in memory and disappear when the master restarts.
- Automatic IPv4 LAN master discovery is implemented; worker heartbeats, liveness tracking, and reconnection are not.
- The scheduler uses fixed-size chunks and round-robin assignment.
- A failed worker task currently fails the overall computation; automatic retries and task recovery are not implemented.
- The implemented execution path is CPU-based. GPU fields in hardware profiles are informational only; GPU task execution is not implemented.
- The current task set is small and experimental; `sum-squares` is the demonstrated distributed workload.
- No authentication or encryption is implemented. Use only on a trusted, isolated network.
- Linux and Android hardware detection are implemented. Other operating systems use a generic fallback profile and have not been validated as release targets.

## Roadmap

1. Zero-configuration LAN discovery.
2. Worker heartbeats, offline detection, and reconnection.
3. CPU benchmarking and performance-aware scheduling.
4. Dynamic chunk sizing and workload monitoring.
5. Task timeouts, retries, and fault tolerance.
6. Additional compute backends, including GPU support.

## Contributing

Issues and pull requests are welcome. The project is under active development, so APIs and protocol details may change between versions.

## License

Apache-2.0. See [LICENSE](LICENSE).

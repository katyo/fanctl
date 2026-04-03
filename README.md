# fanctl

`fanctl` is a replacement for `fancontrol` from `lm_sensors` meant to have a more fine-grained control interface in its config file.

This repo is fork of [mcoffin's fanctl](https://gitlab.com/mcoffin/fanctl) with updates and improvements.

## Motivation

`fancontrol`'s configuration is pretty cryptic, and controlling it is quite hard. [Rust](https://rust-lang.org) was chosen as the implementation language of choice due to the problems that can arise if the program controlling your fans crashes un-cleanly (potentially causing hardware to overheat).

# Usage

## Building

`fanctl` is built with [`cargo`](https://crates.io), the package manager and build system for Rust crates.

```
cargo build --release
```

The built binary will be in `target/release/fanctl`.

## Running

```
fanctl -c <CONFIG_FILE>
```

# Configuration

An example configuration file can be found at [`fanctl.yml`](fanctl.yml).

More detailed information can be found in [the documentation](https://docs.rs/fanctl). The `config` module is a good place to start.

You can build the documentation locally with [`cargo`](https://crates.io).

```bash
# Will build documentation in target/doc
cargo doc --no-deps
```

# License

`fanctl` is released under the GNU General Public License v3.0.

See the [`COPYING`](COPYING) file for more information.

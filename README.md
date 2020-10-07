# Big 2

Networked multiplayer implementation of the card game [Big 2](https://en.wikipedia.org/wiki/Big_two).

The game is written in the Rust and is compatiable with the [Muon programming language](https://github.com/nickmqb/muon) [version of Big2](https://github.com/nickmqb/big2)

[The game rules can be found on Wikipedia](https://en.wikipedia.org/wiki/Big_two).

Platforms: Windows and linux are supported at the moment.


## Build (Windows)

1. Install the latest version of the Rust
2. Clone the big2 repo.
3. build the game `cargo build --release`
4. run `./target/release/big2 -name <yourname> -join <ip/dnsname>[:<port>]`

## Command line arguments

* `-name [yourname]`
* `-join [address]` (join game; address must be IPv4 address, port number is optional: e.g. `127.0.0.1`, `127.0.0.1:1234`, etc.)

For example:
* Join game: `./target/release/big2 -name Saul -join 127.0.0.1`

## Hotkeys

* `Enter`: Play selected cards
* `/`: Pass
* `: Clear selected cards
* `1` to `DEL`: select the cards
* `r`: Ready

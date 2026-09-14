# 0-shell

A minimalist Unix-like shell written in Rust. Implements `echo`, `cd`, `ls`
(`-l`, `-a`, `-F`), `pwd`, `cat`, `cp`, `rm` (`-r`), `mv`, `mkdir`, and `exit`
from scratch using `std::fs` / `std::env`, without spawning any external
binaries.

## Build

```
cargo build --release
```

The binary is produced at `target/release/0-shell`.

## Run

```
./target/release/0-shell
```

## Exit

Type `exit`, or press `Ctrl+D` (EOF).

## Project layout

- `src/main.rs` — the read-eval-print loop, input tokenizer (handles quotes
  for `echo "..."`), and command dispatch.
- `src/commands.rs` — one function per built-in command.

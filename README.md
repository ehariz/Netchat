A decentralized (thus inefficient) chat in rust
===

```
cargo run -- --help
```

## Dev

### Basic test

```
mkfifo fifo
cargo run -- -i fifo -o fifo
```

Have fun talking to yourself !

### Redirect stderr to get debug output

```
mkfifo err
tail -f err
```
From another terminal
```
cargo run --color=always  -- -i fifo -o fifo 2> err
```

### Passing args to the binary through `cargo run`

Use `--` to mark the end of cargo arguments

```
cargo run -- -i a -o b
```
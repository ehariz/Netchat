A fully decentralized (thus inefficient) chat written in rust
===

```
cargo run -- --help
```

## Commands
* `Enter` sends the content of the input field to everyone
* `ctrl+c` exit
* `Ctrl+s` get a snapshot containing every messages sent by every site
* `Ctrl+r` set the private message recipient id to the content of the input field
* `Ctrl+p` sends the content of the input field to the current private recipient
* `Up` scroll messages up
* `Down` scroll messages down
pub struct Opt {

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
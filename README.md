A fully decentralized (thus inefficient) chat written in rust, using named pipes (fifo files) for communication.
===

You can talk to yourself:
```shell
mkfifo fifo
cargo run -- --input fifo --output fifo
```

You can communicate between multiple instances too:
```shell
mkfifo a2b b2a
cargo run -- --input b2a --output a2b --name IamA
# and in another terminal:
cargo run -- -i a2b -o b2a -n IamB
```
You can also use the script `./launch-network.sh` which automate this for 2 instances, and `./launch.py N` does the same for N instances 😱.

But wait, why not have a conversation with your friend which is on another computer ?
```shell
# You
mkfifo in out
cat out | netcat -l 1234 > in1 &
cargo run -- -i in -o out
# Your friend (on another computer)
mkfifo in out
cat out | netcat IP 1234 > in & # replace IP with your IP (not your friend's)
cargo run -- -i in -o out
```
If you have any problem, `rm in out` and `killall netcat` on both computers, then redo the aforementioned steps in the exact same order.

## Arguments

```shell
cargo run -- --help
```

## Commands
* `Enter` sends the content of the input field to everyone
* `Ctrl+c` exit
* `Ctrl+s` get a snapshot containing every messages sent by every site
* `Ctrl+r` set the private message recipient id to the content of the input field or, if let empty, to the id which sent the last private message
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
Netchat
===

_A fully decentralized (thus inefficient) chat written in rust, using named pipes (fifo files) for communication._

**Two-instance communication**

```sh
mkfifo a2b b2a
netchat --input b2a --output a2b --name IamA
# in another terminal:
netchat -i a2b -o b2a -n IamB
```

**Cross-computer communication**

```sh
# You
mkfifo in out
cat out | netcat -l 1234 > in1 &
netchat -i in -o out
# Someone else
mkfifo in out
cat out | netcat IP 1234 > in & # replace IP with your IP
netchat -i in -o out
```

**Troubleshooting:** `rm in out` and `killall netcat` on both computers, then redo the aforementioned steps in the exact same order.

## Commands

* `Enter` sends the content of the input field to everyone
* `Ctrl+c` exit
* `Ctrl+s` get a snapshot containing every messages sent by every site
* `Ctrl+r` set the private message recipient id to the content of the input field or, if let empty, to the id which sent you the last private message
* `Ctrl+p` sends the content of the input field to the current private recipient
* `Up` scroll messages up
* `Down` scroll messages down

# Dev hints

### Arguments

Use `--` to mark the end of cargo arguments and the begining of the program's own.

```shell
cargo run -- --help
```
### Redirect stderr to get debug output

```sh
mkfifo err
tail -f err
# From another terminal
cargo run --color=always  -- -i fifo -o fifo  -l err 2> err
```

### Basic test: one app speaking to itlsef

```
mkfifo fifo
cargo run -- -i fifo -o fifo
```

### Passing args to the binary through `cargo run`

```
cargo run -- -i a -o b
```

### Multiple instances

`./launch-network.sh` automates the fifo creation and routing for 2 instances, and `./launch.py N` does the same for N instances.


# Netchat Overview

*distributed instant messaging*

**Rust, Terminal Interface**

_Mathis Chenuet, Emilien Fugier, Elias Hariz_

```
netchat --help
```

---

## App architecture

```
src
├── main.rs
├── app
│  ├── events.rs
│  └── mod.rs
└── server
   ├── events.rs
   ├── messages.rs
   └── mod.rs
```

There are two modules: `app` and `server`.  
* `app` handle all the user-facing processes, user-input, user-interface...
* `server` handles the routing and all the logic tied the distributed nature of the app.

Each module has his own `events` submodule which provides an `events` object that centralizes all the possible input sources of the module (e.g. for the app the server and the user).

### Event management

Our events are coded as Rust `enums` which can very handily hold various types under a single parent one.

```rust
pub enum Event {
    /// User public message
    UserPublicMessage(String),
    /// User public message
    UserPrivateMessage(AppId, String),
    /// Input from a distant agent (write in a file)
    DistantInput(String),
    /// Shutdown the server
    Shutdown,
    /// Request snapshot from other apps
    GetSnapshot,
    ...
}
```

To pass events around we use  **Multiple Producer Single Consumer (MPSC)**.

```
               
producer 1  +--+
               |
               | events       queue
producer 2  +-----------> +------------+
                   |      |            |
                   |      +------------+
producer 3  +------+      |            |
                          +------------+
                          |            |
                          +------------+ +-----> consumer
```  


```rust
let (tx, rx) = mpsc::channel();

// the transmitter (tx) can be cloned indefinitely and the clone can be use
// to send data to the receiver (rx)
let tx2 = tx.clone();
```

Transmitter can be passed around and use in different threads

```rust
let _thread_handle = {
    let tx = tx.clone();
    thread::spawn(move || loop {
        if let Ok(event) = server_rx.recv() {
            let data = process(event);
            tx.send(data).unwrap();
        }
    })
};
```

To process received events we make heavy use **Pattern matching** which can destructure the events and access the wrapped object.

```rust
loop {
    // events.next() is a blocking operation
    // that consumes the events sent by producers
    match events.next()? {
        Event::UserPublicMessage(message) => {
            process(message);
        }
        Event::UserPrivateMessage(app_id, message) => {
            ...
        }
        Event::Shutdown => {
            ...
        }
    }
}
```

Each service (`app` and `server`) runs in its own thread and communicates by sending messages as mentioned above.

```
App(rx)                             Server(rx)

+-----------+                       +-----------+
|event loop |                       |event loop |
+-----+-----+                       +-----+-----+
      ^                                   ^
      |                                   |
 +----+--+                         +-------------+
 |       |                         |      |      |
 |       |                         |      |      |
 +       +                         +      +      +
User   Server            Distant input   App   Server
```


### Cross site messages

(serde)[https://github.com/serde-rs/serde] is used to serialize and deserialize Rust object to and from strings, the strings are then sent through the pipes for others to read.

Message are serialized to json in order to be human readable, for a production application, we would use a less verbose format (switching is transparent thanks to serde).

### User Interface

The interface is built using [tui-rs](https://github.com/fdehau/tui-rs) with a [termion](https://github.com/redox-os/termion) backend.

---

## Snapshot and message history 

A snapshot can be asked for, it will contain all the information available on the netwrok.

The snapshot is then processed into an hsitory which is much more human-friendly.

## Topology-agnostic protocol

Each site broadcasts every received message to ensure propagation.

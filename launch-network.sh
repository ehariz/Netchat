#!/bin/sh

mkfifo /tmp/launch.network.1
mkfifo /tmp/launch.network.2

cd $(dirname $0)

x-terminal-emulator -e 'sh -c "RUST_LOG=project=trace cargo run -- --input /tmp/launch.network.1 --output /tmp/launch.network.2 -n one 2>err"' &
x-terminal-emulator -e 'sh -c "cargo run -- --input /tmp/launch.network.2 --output /tmp/launch.network.1 -n two"' &
wait

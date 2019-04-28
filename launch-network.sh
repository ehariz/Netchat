#!/bin/sh

mkfifo /tmp/launch.network.1
mkfifo /tmp/launch.network.2

cd $(dirname $0)

uxterm -e 'sh -c "cargo run -- /tmp/launch.network.1 /tmp/launch.network.2"' &
uxterm -e 'sh -c "cargo run -- /tmp/launch.network.2 /tmp/launch.network.1"' &

wait

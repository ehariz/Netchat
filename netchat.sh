#!/bin/sh

# Script to test locally remote connections with netcat

rm in1 out1 in2 out2
mkfifo in1 out1 in2 out2

cat out1 | netcat -l 1234 > in1 &
cat out2 | netcat localhost 1234 > in2 &

x-terminal-emulator -e 'cargo run -- -i in1 -o out1 -n ME' 
x-terminal-emulator -e 'cargo run -- -i in2 -o out2 -n FRIEND'

wait

#! /bin/sh
kill $(ps aux | grep 'target/debug/project' | awk '{print $2}')

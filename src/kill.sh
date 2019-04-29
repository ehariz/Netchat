#! /bin/sh
kill $(ps aux | grep 'project' | awk '{print $2}')

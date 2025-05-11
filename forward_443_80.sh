#!/usr/bin/env bash

sudo whoami
if [ "$1" == "d" ]; then
    sudo killall socat
    exit
fi
sudo socat TCP-LISTEN:443,fork,reuseaddr TCP:127.0.0.1:8443 &
sudo socat TCP-LISTEN:80,fork,reuseaddr TCP:127.0.0.1:3000 &

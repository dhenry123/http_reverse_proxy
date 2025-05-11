#!/usr/bin/env bash

sudo setcap 'cap_net_bind_service=+ep' target/release/http_reverse_proxy

#!/bin/sh
cd /tmp/1521-httpx
env HOME=./home PATH=./bin:$PATH CARGO_HOME=$REAL_HOME/.cargo RUSTUP_HOME=$REAL_HOME/.rustup GOPATH=$REAL_HOME/go "$MERL_BIN" "$@"
sleep 6

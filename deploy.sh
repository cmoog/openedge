#!/usr/bin/env fish

docker build -t openedge:latest .; or exit 1
flyctl deploy --local-only; or exit 1

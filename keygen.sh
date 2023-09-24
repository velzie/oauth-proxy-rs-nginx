#!/bin/sh
tr -dc '[:alpha:]' </dev/urandom | fold -w "${1:-50}" | head -n 1 >keys/secret
echo "created key at $(realpath keys/secret)!"

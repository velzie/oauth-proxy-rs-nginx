#!/bin/sh
mkdir -p keys/secret.pem
tr -dc '[:alpha:]' </dev/urandom | fold -w "${1:-50}" | head -n 1 >keys/secret.pem
echo "created key at $(realpath keys/secret.pem)"

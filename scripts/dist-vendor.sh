#!/bin/sh

export DIST="$1"
export SOURCE_ROOT="$2"

cd "$SOURCE_ROOT" || exit 1

mkdir "$DIST"/.cargo
cargo vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > $DIST/.cargo/config

# Move vendor into dist tarball directory
mv vendor "$DIST"
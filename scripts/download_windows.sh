#!/bin/sh
build_type=${1:-release}
cargo run --profile $build_type --bin cli

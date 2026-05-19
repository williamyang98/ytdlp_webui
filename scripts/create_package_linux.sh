#!/bin/sh
path=${1:-package}
build_type=${2:-release}
rm -rf $path/
mkdir -p $path/
cp ./target/$build_type/server $path/
cp ./README.md $path/
cp ./LICENSE $path/
cp ./scripts/.env_linux $path/.env
cp -rf ./static/ $path/
cp -rf ./scripts/ $path/
cp -rf ./docs/ $path/

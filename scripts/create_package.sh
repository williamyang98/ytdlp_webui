#!/bin/sh
path=${1:-package}
build_type=${2:-release}
exec_name=${3:-ytdlp_server.exe}
rm -rf $path/
mkdir -p $path/
cp ./target/$build_type/$exec_name $path/
cp ./README.md $path/
cp ./LICENSE $path/
cp -rf ./static/ $path/
cp -rf ./bin/ $path/
cp -rf ./scripts/ $path/
cp -rf ./docs/ $path/

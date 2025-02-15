#!/bin/sh
# Download files
mkdir ./bin/
cd ./bin
curl -fLo ./yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/download/2025.01.26/yt-dlp.exe &
curl -fLo ./ffmpeg.7z https://github.com/GyanD/codexffmpeg/releases/download/7.0.1/ffmpeg-7.0.1-essentials_build.7z &
wait
# Verify hash
set -e
echo e19115321897a27c2fcf73d3b23d5139847a8c4fd4792eecce1712bfd9accd05 ffmpeg.7z | sha256sum --check
echo 423eec9b60ab7910d97eb74cdb5daea90128850752d4aad6ccabaf8648d6387c yt-dlp.exe | sha256sum --check
# Unzip
7z x ./ffmpeg.7z -offmpeg -y
cp ./ffmpeg/ffmpeg-7.0.1-essentials_build/bin/ffmpeg.exe ./ffmpeg.exe
# Cleanup
rm ./ffmpeg.7z
rm -rf ./ffmpeg

#!/bin/sh
# Download files
mkdir ./bin/
cd ./bin
curl -fLo ./yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/download/2024.05.27/yt-dlp.exe
curl -fLo ./ffmpeg.7z https://github.com/GyanD/codexffmpeg/releases/download/7.0.1/ffmpeg-7.0.1-essentials_build.7z &
wait
# Verify hash
set -e
echo e19115321897a27c2fcf73d3b23d5139847a8c4fd4792eecce1712bfd9accd05 ffmpeg.7z | sha256sum --check
echo e96f6348244306ac999501b1e8e2b096b8a57f098c3b2b9ffe64b2107039e0ae yt-dlp.exe | sha256sum --check
# Unzip
7z x ./ffmpeg.7z -offmpeg -y
cp ./ffmpeg/ffmpeg-7.0.1-essentials_build/bin/ffmpeg.exe ./ffmpeg.exe
# Cleanup
rm ./ffmpeg.7z
rm -rf ./ffmpeg

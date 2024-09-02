#!/bin/sh
# Download files
mkdir ./bin/
cd ./bin
curl -fLo ./yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/download/2024.08.06/yt-dlp.exe &
curl -fLo ./ffmpeg.7z https://github.com/GyanD/codexffmpeg/releases/download/7.0.1/ffmpeg-7.0.1-essentials_build.7z &
wait
# Verify hash
set -e
echo e19115321897a27c2fcf73d3b23d5139847a8c4fd4792eecce1712bfd9accd05 ffmpeg.7z | sha256sum --check
echo 468a6f8bf1d156ad173e000a40f696d4fbd69c5aa7360229329b9063a388e7d0 yt-dlp.exe | sha256sum --check
# Unzip
7z x ./ffmpeg.7z -offmpeg -y
cp ./ffmpeg/ffmpeg-7.0.1-essentials_build/bin/ffmpeg.exe ./ffmpeg.exe
# Cleanup
rm ./ffmpeg.7z
rm -rf ./ffmpeg

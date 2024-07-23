#!/bin/sh
# Download files
mkdir ./bin/
cd ./bin
curl -fLo ./yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/download/2024.07.16/yt-dlp.exe &
curl -fLo ./ffmpeg.7z https://github.com/GyanD/codexffmpeg/releases/download/7.0.1/ffmpeg-7.0.1-essentials_build.7z &
wait
# Unzip
7z x ./ffmpeg.7z -offmpeg -y
cp ./ffmpeg/ffmpeg-7.0.1-essentials_build/bin/ffmpeg.exe ./ffmpeg.exe
# Verify hash
set -e
echo d7b51e782c79f564d6e33907b17b010f01634c00e3c42559975cbc7a82982f8f ffmpeg.exe | sha256sum --check
echo f01b37ca4f3e934208a5439d1ec8ae49a18f2be9f68fec5e3cfed08cc38b3275 yt-dlp.exe | sha256sum --check
# Cleanup
rm ./ffmpeg.7z
rm -rf ./ffmpeg

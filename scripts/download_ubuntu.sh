#!/bin/sh
# Install yt-dlp
mkdir ./bin/
cd ./bin/
curl -fLo ./yt-dlp https://github.com/yt-dlp/yt-dlp/releases/download/2025.01.26/yt-dlp_linux
set -e
echo 0aa0afe3d2b32c047b73083f3c8e56081d71bb33fe047357820d51d153d1d54f yt-dlp | sha256sum --check
sudo chmod 777 ./yt-dlp
# Install ffmpeg
sudo apt-get --yes install ffmpeg

#!/bin/sh
# Install yt-dlp
mkdir ./bin/
curl -fLo ./bin/yt-dlp https://github.com/yt-dlp/yt-dlp/releases/download/2024.05.27/yt-dlp_linux
set -e
echo af0570b5e60196a1785a12e7f48fc7cb7b5745b0bb9870ca2fe6ed90ddd80b46 ./bin/yt-dlp | sha256sum --check
sudo chmod 777 ./bin/yt-dlp
# Install ffmpeg
sudo apt-get --yes install ffmpeg

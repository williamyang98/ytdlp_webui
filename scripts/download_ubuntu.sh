#!/bin/sh
# Install yt-dlp
mkdir ./bin/
cd ./bin/
curl -fLo ./yt-dlp https://github.com/yt-dlp/yt-dlp/releases/download/2024.07.16/yt-dlp_linux
set -e
echo a6b840e536014ce7b2c7c40b758080498ed5054aa96979e64fcc369752cdc8d3 yt-dlp | sha256sum --check
sudo chmod 777 ./yt-dlp
# Install ffmpeg
sudo apt-get --yes install ffmpeg

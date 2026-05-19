#!/bin/sh
sudo apt-get --yes install ffmpeg python3 curl
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash
. $HOME/.nvm/nvm.sh
nvm install --lts
pip install yt-dlp

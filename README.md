# Introduction
[![x86-windows](https://github.com/williamyang98/ytdlp_webui/actions/workflows/x86-windows.yml/badge.svg)](https://github.com/williamyang98/ytdlp_webui/actions/workflows/x86-windows.yml)
[![x86-linux](https://github.com/williamyang98/ytdlp_webui/actions/workflows/x86-linux.yml/badge.svg)](https://github.com/williamyang98/ytdlp_webui/actions/workflows/x86-linux.yml)

Download and convert Youtube videos into audio clips. Has a web UI client that wraps around ```yt-dlp``` and ```ffmpeg``` and caches downloaded and transcoded files locally.

## Usage
1. Start the server: ```server --url 127.0.0.1 --port 8080```.
2. Access the webpage with your browser: ```http://localhost:8080```.
3. Copy and paste video link into URL bar.
4. Press ```Request``` button.
5. Wait for download and trancode to finish.
6. Press ```Download``` button to get audio clip.

## Gallery
![Screenshot](./docs/screenshot_webpage.png)

## Building from source
### Building website
1. Follow the instructions to install node.js and npm from [https://docs.npmjs.com/downloading-and-installing-node-js-and-npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) 
2. Enter the ```frontend``` directory
3. Install npm packages: ```npm install```
4. Serve website in development mode: ```npm run dev```
5. Build website to ```static``` folder in top level directory: ```npm run build```
### Building server
1. Download and install rust from [https://rust-lang.org/tools/install/](https://rust-lang.org/tools/install/)
2. Enter the root directory
3. Build programs: ```cargo build -r```
4. Download binaries: ```cargo run -r --bin cli```
5. Download packages using ```./scripts/download_*.sh``` for your platform if required
6. Copy environment file ```./scripts/.env_*``` for your platform and rename it to ```.env``` in the root folder
7. Download binaries: ```cargo run -r --bin cli```
8. Run server: ```cargo run -r --bin server -- --enable-cors```
### Note about cross origin resource sharing (CORS)
- Passing the argument ```--enable-cors``` when running the server enables cross origin resource sharing
- This allows for the frontend development server for the website to connect to the backend server
- You can avoid passing this flag when accessing the website from the backend server

## Getting a Youtube data api v3 key
- You may have to provide your own Youtube data api v3 key if the provided public key is overused or invalidated
- Follow the instructions here to get your own api key [https://developers.google.com/youtube/v3/getting-started](https://developers.google.com/youtube/v3/getting-started)
- Then place it inside your ```.env``` file in the root folder as ```YOUTUBE_API_KEY=INSERT_YOUR_YOUTUBE_API_KEY_HERE``` 

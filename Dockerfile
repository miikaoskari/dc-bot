FROM alpine:latest
RUN apk add --no-cache nodejs npm yt-dlp
WORKDIR /app
COPY . /app
RUN npm install
RUN node deployCommands.js
CMD ["node", "index.js"]

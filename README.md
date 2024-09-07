# discord bot

bot being run in a private server

commands:
- video 

## install
install deps

```bash
npm install
```

add config.json containing `token` and `clientId`.

## usage
register commands:

```bash
node deployCommands.js
```

run bot:

```bash
node index.js
```

## docker

```bash
docker buildx build -t dc-bot .
docker run -v ./config.json:/app/config.json dc-bot
```
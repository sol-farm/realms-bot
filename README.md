# realms-bot

Discord bot for monitoring Realms DAO proposals, initially targetting Mint Governance account types.

# usage

## No Docker

```shell
$> make build-cli
$> ./realms-bot config new
$> # populate the configuration file with relevant information
$> ./realms-bot config seed-database # seed the embedded database with existing governance data
$> ./realms-bot discord # starts the discord bot
```

## Docker

After running the following command the compiled  docker image will be saved to `realms_bot.tar.gz`.

```shell
$> make build-docker
```
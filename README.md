# realms-bot

Discord bot for monitoring Realms DAO proposals, initially targetting Mint Governance account types.

# usage

## No Docker

> Requires nightly installation of rust

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

## `realms_sdk`

The `realms_sdk` folder contains the source code for the `tulip-realms-sdk` crate, which is a wrapper around a sled embedded database, with support for storing realm, governance, and proposal accounts, as well as simple queries. For documentation on how to use this crate, see the tests, or the `discord` folder.
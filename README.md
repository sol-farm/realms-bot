# realms-bot

Discord bot for monitoring Realms DAO proposals, initially targetting Mint Governance account types.

# usage

```shell
$> make build-cli
$> ./realms-bot config new
$> # populate the configuration file with relevant information
$> ./realms-bot config seed-database # seed the embedded database with existing governance data
$> ./realms-bot discord # starts the discord bot
```

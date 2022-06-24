# realms-bot

Discord bot for monitoring Realms DAO proposals, initially targetting Mint Governance account types.

# Limitations

At the moment only mint governance realms are supported, however it is relatively simple to add support for new governance types. If you wish to have a new governance type supported, please open a github issue.

# Features

* Monitors for newly submitted proposals that are in the `Voting` state.
* Once a proposal leaves the `Voting` state, it is no longer tracked by the bot.
* Disk based persistence using `sled`
* Periodic reminders about actively voting proposals

# Usage

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

> Requires a docker installation that supports docker buildkit

After running the following command the compiled  docker image will be saved to `realms_bot.tar.gz`.

```shell
$> make build-docker
```

## Configuration

A fully populated configuration file that is currently used for the Tulip discord is below and can be used as a reference for your own discord server. You will need to populate the `discord.bot_token` field with a bot token that has access to your discord server, and has the  `GUILD_MESSAGES` and `MESSAGE_CONTENT` gateway intents enabled. The `discord.status_channel` field is used to indicate which discord channel the bot should post messages to.

Additionally if you self-host a ui for your DAO, replace `discord.ui_base_url` with your self-hosted ui, for example with Solend's UI you would fill in `https://govern.solend.fi/dao/SLND`. If you do not host your on ui leave the templated value, replacing `<realm-id>` with whatever realm account your DAO uses. For example if your realm account is `123abc` set `discord.ui_base_url` to `https://realms.today/dao/123abc`.

To configure the governance realm which is monitored, you will need to replace all of the `realm_info.*` fields with the appropriate values for your realm. 


```yaml
---
discord:
  bot_token: <your-bot-token-here>
  status_channel: <your-status-channel>
  # how often in seconds the discord bot should check for new proposals
  worker_loop_frequency: 10
  # used for linking to the proposal within embed messages
  ui_base_url: "https://realms.today/dao/<realm-id>"
  # how often in hours to post a reminder message that a proposal can still be voted on
  notification_frequency: 6
db_opts:
  compression_factor: ~
  debug: false
  mode: ~
  path: ./realms_bot.db
  system_page_cache: ~
realm_info:
  realm_key: 413KSeuFUBSWDzfjU9BBqBAWYKmoR8mncrhV84WcGNAk
  council_mint_key: EzSjCzCPwpchdQVaGJZYpgDNagzasKFVGJ66Dmut26FL
  community_mint_key: STuLiPmUCUtG1hQcwdc9de9sjYhVsYoucCiWqbApbpM
  governance_key: 86ceNv5dy2Q7EYBmy5iPkuMGTeRBa8gMm7kmA96N4MQG
log_file: realms_bot.log
debug_log: false
rpc_url: "http://haproxy:8899"
```

### Docker Compose Configuration

For docker compose the only notable configuration difference is that `db_opts.path` must be the path of the database directory when it is mounted within docker.

For example given the following docker compose file, you would want to update the `realms_config.yaml` file to set the the `db_opts.path` field to `/tmp/realms_bot.db`


```yaml
version: "3.5"
services:
  realms:
    image: realms-bot:latest
    command: --config  /tmp/config.yaml discord
    restart: always
    volumes:
      - ./realms_config.yaml:/tmp/config.yaml
      - ./realms_bot.db:/tmp/realms_bot.db:rw
```

When running the bot within docker for the first time, you should use the following configuration, which is replaced by the above configuration after the database is seeded:


```yaml
version: "3.5"
services:
  realms:
    image: realms-bot:latest
    command: --config  /tmp/config.yaml config seed-databse
    restart: always
    volumes:
      - ./realms_config.yaml:/tmp/config.yaml
      - ./realms_bot.db:/tmp/realms_bot.db:rw
```

```sh
$> docker-compose up realms # seeds the database
$> # swap out the config file updating `command` to `command: --config /tmp/config.yaml discord`
$> docker-compose up -d realms # starts a detatched docker container 
```
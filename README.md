# GPT Discord Bot

## Project Description

This is a Discord bot written in Rust that uses OpenAI's GPT model to generate responses.

## Installation

First, you need to install Rust and Cargo. You can get the installation guide from the [official Rust website](https://www.rust-lang.org/tools/install).

Then, clone this repository and navigate into the project directory:

```bash
git clone https://github.com/kasugamirai/gpt_discord_bot
```

```bash
cd gpt-discord-bot
```

## how to use

edit the run.sh file and set the `DISCORD_BOT_TOKEN` and `OPENAI_API_KEY` environment.

```bash
sh build.sh
```

```bash
cd output && sh run.sh
```

## example
To interact with the bot, start your message with a period. For example:

```
.hello world
```

This will prompt the bot to respond. Enjoy chatting with your new AI-powered Discord bot!
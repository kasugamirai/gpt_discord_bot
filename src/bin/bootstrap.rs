use gpt_discord_bot::discord;
use serenity::client::Client;
use std::env;

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("please provide a valid env DISCORD_TOKEN");
}

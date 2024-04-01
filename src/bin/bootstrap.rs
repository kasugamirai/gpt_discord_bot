use gpt_discord_bot::ChatHandler;
use serenity::prelude::*;
use std::env;
use std::path::Path;

use gpt_discord_bot::Handler;

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    // Load environment variables from .env file
    if Path::new(".env").exists() {
        match dotenv::dotenv() {
            Ok(_) => {}
            Err(_) => println!("Failed to load .env file"),
        }
    }
    // Get the discord token from the environment
    let discord_token = match env::var("DISCORD_TOKEN") {
        Ok(val) => val,
        Err(_) => panic!("Expected a discord token in the environment"),
    };
    // Get the OpenAI API key from the environment
    let gpt_api_key = match env::var("OPENAI_API_KEY") {
        Ok(val) => val,
        Err(_) => panic!("Expected a OPEN AI key in the environment"),
    };
    // Set the intents for the bot
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES;

    // Create a new client with the discord token
    let mut client = match Client::builder(&discord_token, intents)
        .event_handler(Handler::new(&gpt_api_key))
        .await
    {
        Ok(client) => client,
        Err(_) => panic!("Err creating client"),
    };

    // Start listening for events
    println!("Bot is now running. Press Ctrl+C to stop.");
    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}

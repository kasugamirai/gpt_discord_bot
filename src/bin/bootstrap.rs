use serenity::{client::Client, prelude::*};
use std::env;
use tokio;

use dotenv;

use gpt_discord_bot::discord::bot::Handler;

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    // Load environment variables from .env file
    dotenv::dotenv().expect("Failed to load .env file");
    // Get the discord token from the environment
    let discord_token =
        env::var("DISCORD_TOKEN").expect("Expected a discord token in the environment");
    // Get the OpenAI API key from the environment
    let gpt_api_key =
        env::var("OPENAI_API_KEY").expect("Expected a OPEN AI key in the environment");
    // Set the intents for the bot
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES;

    // Create a new client with the discord token
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(Handler::new(gpt_api_key).await.unwrap())
        .await
        .expect("Error creating client");

    // Start listening for events
    println!("Bot is now running. Press Ctrl+C to stop.");
    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}

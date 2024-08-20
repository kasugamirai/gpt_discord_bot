use gpt_discord_bot::create_client;
use gpt_discord_bot::get_env;
use gpt_discord_bot::load_environment_variables;
use gpt_discord_bot::Handler;
use serenity::all::GatewayIntents;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize the logger with tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    load_environment_variables();

    // Set the intents for the bot
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES;

    let handler_result = Handler::new(&get_env(
        "OPENAI_API_KEY",
        "Expected a OPEN AI key in the environment",
    ));
    let handler = match handler_result {
        Ok(handler) => handler,
        Err(e) => panic!("Error creating handler: {}", e),
    };

    // Create a new client with the discord token
    let mut client = create_client(
        &get_env(
            "DISCORD_TOKEN",
            "Expected a discord token in the environment",
        ),
        intents,
        handler,
    )
    .await;

    // Start listening for events
    info!("Bot is now running. Press Ctrl+C to stop.");
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

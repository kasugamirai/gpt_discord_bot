use chatgpt::client::ChatGPT;
use chatgpt::config::{ChatGPTEngine, ModelConfiguration, ModelConfigurationBuilder};
use chatgpt::types::ResponseChunk;
use futures::StreamExt;
use serenity::async_trait;
use serenity::builder::EditMessage;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::GatewayIntents;
use serenity::Client;
use std::time::Duration;
use thiserror::Error;
use tokio::time::interval;
use tracing::error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ModelConfigurationBuilderError(#[from] chatgpt::config::ModelConfigurationBuilderError),

    #[error(transparent)]
    ChatGPT(#[from] chatgpt::err::Error),
}

type Result<T> = std::result::Result<T, Error>;

fn init_gpt_client(api_key: &str) -> Result<ChatGPT> {
    let config: ModelConfiguration = ModelConfigurationBuilder::default()
        .engine(ChatGPTEngine::Gpt4)
        .timeout(Duration::from_secs(500))
        .build()?;

    let client = ChatGPT::new_with_config(api_key, config)?;
    Ok(client)
}

// The Handler struct is the event handler for the bot.
pub struct Handler {
    pub gpt_client: ChatGPT,
}

impl Handler {
    pub fn new(api_key: &str) -> Result<Self> {
        let gpt_client = init_gpt_client(api_key)?;
        Ok(Self { gpt_client })
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !filter_msg(&msg) {
            return;
        }

        let prompt = &msg.content[1..];
        let mut stream = match self.gpt_client.send_message_streaming(prompt).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Error sending message to ChatGPT: {:?}", e);
                return;
            }
        };

        let processing_message = match msg.channel_id.say(&ctx.http, "Processing...").await {
            Ok(message) => message,
            Err(why) => {
                error!("Error sending message: {:?}", why);
                return;
            }
        };

        let mut result = String::new();
        let mut interval = interval(Duration::from_millis(900));

        loop {
            tokio::select! {
                chunk = stream.next() => {
                    if let Some(chunk) = chunk {
                        if let ResponseChunk::Content { delta, response_index: _ } = chunk {
                                result.push_str(&delta);
                        }
                    } else {
                        // Stream has ended, break the loop
                        break;
                    }
                },
                _ = interval.tick() => {
                    if !result.is_empty() {
                        let edit = EditMessage::default().content(&result);
                        if let Err(why) = msg.channel_id.edit_message(&ctx.http, processing_message.id, edit).await {
                            error!("Error editing message: {:?}", why);
                        }
                    }
                }
            }
        }

        // Ensure any remaining content is also sent
        if !result.is_empty() {
            let edit = EditMessage::default().content(&result);
            let _ = msg
                .channel_id
                .edit_message(&ctx.http, processing_message.id, edit)
                .await;
        }
    }
}

fn filter_msg(msg: &Message) -> bool {
    if msg.author.bot || !msg.content.starts_with('.') {
        return false;
    }
    true
}

pub async fn create_client(
    discord_token: &str,
    intents: GatewayIntents,
    handler: Handler,
) -> Client {
    match Client::builder(discord_token, intents)
        .event_handler(handler)
        .await
    {
        Ok(client) => client,
        Err(e) => panic!("Err creating client{}", e),
    }
}

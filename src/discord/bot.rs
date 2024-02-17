use chatgpt::prelude::*;
use futures::StreamExt;
use serenity::client::{Context, EventHandler};
use serenity::{async_trait, builder::EditMessage, model::channel::Message};
use std::time::Duration;
use tokio::time::interval;

// The Handler struct is the event handler for the bot.
pub struct Handler {
    pub gpt_client: ChatGPT,
}

// The Handler struct is the event handler for the bot.
impl Handler {
    pub async fn new(api_key: String) -> Result<Self> {
        let config: ModelConfiguration = ModelConfigurationBuilder::default()
            .engine(ChatGPTEngine::Gpt4)
            .build()
            .unwrap_or_else(|e| {
                log::error!("Failed to build ModelConfiguration: {}", e);
                ModelConfiguration::default()
            });
        let gpt_client = ChatGPT::new_with_config(api_key, config)?;
        Ok(Self { gpt_client })
    }
}

use tokio::select;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.content.starts_with(".") {
            let prompt = msg.content[1..].trim();
            let future = self.gpt_client.send_message_streaming(prompt);
            let mut stream = match future.await {
                Ok(stream) => stream,
                Err(e) => {
                    log::error!("Error sending message to ChatGPT: {:?}", e);
                    return;
                }
            };

            let processing_message = match msg.channel_id.say(&ctx.http, "Processing...").await {
                Ok(message) => message,
                Err(why) => {
                    log::error!("Error sending message: {:?}", why);
                    return;
                }
            };

            let mut result = String::new();
            let mut interval = interval(Duration::from_millis(800));

            loop {
                select! {
                    chunk = stream.next() => {
                        if let Some(chunk) = chunk {
                            match chunk {
                                ResponseChunk::Content { delta, response_index: _ } => {
                                    result.push_str(&delta);
                                },
                                _ => {}
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
                                log::error!("Error editing message: {:?}", why);
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
}

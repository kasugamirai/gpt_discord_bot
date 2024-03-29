use chatgpt::prelude::*;
use futures::StreamExt;
use serenity::client::{Context, EventHandler as SerenityEventHandler};
use serenity::{async_trait, builder::EditMessage, model::channel::Message};
use std::time::Duration;
use tokio::select;
use tokio::time::interval;

// Define a trait representing handler behavior
#[async_trait]
pub trait ChatHandler {
    // Create a new instance of the handler
    fn new(api_key: &String) -> Self
    where
        Self: Sized;

    // Process an incoming message and return a response
    async fn process_message(&self, msg: &Message) -> Option<String>;
}

// Handler struct implementing the ChatHandler trait
pub struct Handler {
    pub gpt_client: ChatGPT,
}

#[async_trait]
impl ChatHandler for Handler {
    fn new(api_key: &String) -> Self {
        let config: ModelConfiguration = ModelConfigurationBuilder::default()
            .engine(ChatGPTEngine::Gpt4)
            .timeout(Duration::from_secs(50))
            .build()
            .unwrap_or_else(|e| {
                log::error!("Failed to build ModelConfiguration: {}", e);
                ModelConfiguration::default()
            });
        let gpt_client = ChatGPT::new_with_config(api_key, config).unwrap();
        Self { gpt_client }
    }

    async fn process_message(&self, msg: &Message) -> Option<String> {
        if msg.author.bot || !msg.content.starts_with(".") {
            return None;
        }

        let prompt = msg.content[1..].trim();
        let future = self.gpt_client.send_message_streaming(prompt);
        let mut stream = future.await.ok()?;
        let mut result = String::new();
        let mut interval = interval(Duration::from_millis(900));

        loop {
            select! {
                chunk = stream.next() => {
                    if let Some(chunk) = chunk {
                        if let ResponseChunk::Content { delta, response_index: _ } = chunk {
                            result.push_str(&delta);
                        }
                    } else {
                        break;
                    }
                },
                _ = interval.tick() => {}
            }
        }

        Some(result)
    }
}

#[async_trait]
impl SerenityEventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(result) = self.process_message(&msg).await {
            let processing_future = msg.channel_id.say(&ctx.http, "Processing...");

            match processing_future.await {
                Ok(processing_message) => {
                    if !result.is_empty() {
                        let edit = EditMessage::default().content(&result);
                        if let Err(why) = msg
                            .channel_id
                            .edit_message(&ctx.http, processing_message.id, edit)
                            .await
                        {
                            log::error!("Error editing message: {:?}", why);
                        }
                    }
                }
                Err(why) => log::error!("Error sending message: {:?}", why),
            }
        }
    }
}

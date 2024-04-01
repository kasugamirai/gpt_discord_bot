use chatgpt::client::ChatGPT;
use chatgpt::config::{ChatGPTEngine, ModelConfiguration, ModelConfigurationBuilder};
use chatgpt::types::ResponseChunk;
use core::fmt;
use futures::StreamExt;
use serenity::client::{Context, EventHandler as SerenityEventHandler};
use serenity::{async_trait, builder::EditMessage, model::channel::Message};
use std::time::Duration;
use tokio::select;
use tokio::time::interval;

#[derive(Debug)]
pub enum Error {
    ModelConfigurationBuilderError(chatgpt::config::ModelConfigurationBuilderError),
    ChatGPT(chatgpt::err::Error),
}

impl From<chatgpt::config::ModelConfigurationBuilderError> for Error {
    fn from(err: chatgpt::config::ModelConfigurationBuilderError) -> Self {
        Error::ModelConfigurationBuilderError(err)
    }
}

impl From<chatgpt::err::Error> for Error {
    fn from(err: chatgpt::err::Error) -> Self {
        Error::ChatGPT(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ModelConfigurationBuilderError(err) => {
                write!(f, "ModelConfigurationBuilderError: {}", err)
            }
            Error::ChatGPT(err) => write!(f, "ChatGPTError: {}", err),
        }
    }
}

// Define a trait representing handler behavior
#[async_trait]
pub trait ChatHandler {
    // Create a new instance of the handler
    fn new(api_key: &String) -> Result<Self, Error>
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
    fn new(api_key: &String) -> Result<Self, Error> {
        let config: ModelConfiguration = ModelConfigurationBuilder::default()
            .engine(ChatGPTEngine::Gpt4)
            .timeout(Duration::from_secs(50))
            .build()?;
        let gpt_client = ChatGPT::new_with_config(api_key, config)?;
        Ok(Handler { gpt_client })
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

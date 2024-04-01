use chatgpt::client::ChatGPT;
use chatgpt::config::{ChatGPTEngine, ModelConfiguration, ModelConfigurationBuilder};
use chatgpt::types::ResponseChunk;
use futures::StreamExt;
use serenity::async_trait;
use serenity::builder::EditMessage;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use std::fmt;
use std::time::Duration;
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

fn init_gpt_client(api_key: &str) -> Result<ChatGPT, Error> {
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

// The Handler struct is the event handler for the bot.
impl Handler {
    pub fn new(api_key: &str) -> Result<Self, Error> {
        let gpt_client = init_gpt_client(api_key)?;
        Ok(Self { gpt_client })
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message<'a>(&'a self, ctx: Context, msg: Message) {
        if msg.author.bot || !msg.content.starts_with(".") {
            return;
        }

        let prompt = &msg.content[1..];
        let mut stream = match self.gpt_client.send_message_streaming(prompt).await {
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
        let mut interval = interval(Duration::from_millis(900));

        loop {
            tokio::select! {
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

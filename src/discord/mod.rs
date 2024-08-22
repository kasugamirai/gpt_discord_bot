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
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, error};

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
        let stream = match self.gpt_client.send_message_streaming(prompt).await {
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

        let res: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let res_clone = res.clone();
        let http_clone = ctx.http.clone();
        let mut interval: tokio::time::Interval = interval(Duration::from_millis(900));

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                let res = res_clone.lock().await.clone();
                let edit = EditMessage::default().content(&res);
                if let Err(why) = processing_message
                    .channel_id
                    .edit_message(&http_clone, processing_message.id, edit)
                    .await
                {
                    error!("Error editing message: {:?}", why);
                }
            }
        });

        stream
            .for_each(|each| {
                let result: Arc<Mutex<String>> = res.clone();
                {
                    //let value = ctx.http.clone();
                    async move {
                        if let ResponseChunk::Content {
                            delta,
                            response_index: _,
                        } = each
                        {
                            debug!("{}", delta);
                            let mut res_ref = result.lock().await;
                            res_ref.push_str(&delta);
                        }
                    }
                }
            })
            .await;

        let result = res.lock().await.clone();

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

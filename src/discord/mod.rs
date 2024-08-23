use chatgpt::client::ChatGPT;
use chatgpt::config::{ChatGPTEngine, ModelConfigurationBuilder};
use chatgpt::types::ResponseChunk;
use futures::StreamExt;
use serenity::async_trait;
use serenity::builder::EditMessage;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::GatewayIntents;
use serenity::Client;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::{interval, Interval};
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
    let config = ModelConfigurationBuilder::default()
        .engine(ChatGPTEngine::Gpt4)
        .timeout(Duration::from_secs(500))
        .build()?;

    ChatGPT::new_with_config(api_key, config).map_err(Error::from)
}

pub struct Handler {
    gpt_client: ChatGPT,
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
        if !should_process_message(&msg) {
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

        let response_content = Arc::new(Mutex::new(String::new()));
        let res_clone = Arc::clone(&response_content);
        let http_clone = Arc::new(ctx.http.clone());
        let interval = interval(Duration::from_millis(500));

        tokio::spawn(update_message_interval(
            Arc::clone(&http_clone),
            processing_message.clone(),
            Arc::clone(&res_clone),
            interval,
        ));

        stream
            .for_each(|chunk| {
                let res_clone = Arc::clone(&res_clone);
                async move {
                    if let ResponseChunk::Content { delta, .. } = chunk {
                        debug!("{}", delta);
                        let mut response = res_clone.lock().await;
                        response.push_str(&delta);
                    }
                }
            })
            .await;

        let final_content = response_content.lock().await.clone();

        if !final_content.is_empty() {
            if let Err(why) = processing_message
                .channel_id
                .edit_message(
                    &ctx.http,
                    processing_message.id,
                    EditMessage::new().content(&final_content),
                )
                .await
            {
                error!("Error editing final message: {:?}", why);
            }
        }
    }
}

fn should_process_message(msg: &Message) -> bool {
    !msg.author.bot && msg.content.starts_with('.')
}

async fn update_message_interval(
    http: Arc<serenity::http::Http>,
    processing_message: Message,
    res_clone: Arc<Mutex<String>>,
    mut interval: Interval,
) {
    loop {
        interval.tick().await;
        let content = res_clone.lock().await.clone();
        if let Err(why) = processing_message
            .channel_id
            .edit_message(
                &http,
                processing_message.id,
                EditMessage::new().content(&content),
            )
            .await
        {
            error!("Error editing message: {:?}", why);
        }
    }
}

pub async fn create_client(
    discord_token: &str,
    intents: GatewayIntents,
    handler: Handler,
) -> Client {
    Client::builder(discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating client")
}

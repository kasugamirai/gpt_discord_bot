use chatgpt::prelude::*;
use futures::StreamExt;
use std::io::{stdout, Write};

use serenity::{
    async_trait, client,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

pub struct Handler {
    openai_api_key: String,
}

impl Handler {
    pub fn new(openai_api_key: String) -> Self {
        Self { openai_api_key }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.content.starts_with("!gpt") {
            let prompt = msg.content[4..].trim();
            let key = self.openai_api_key.clone();
            let gpt_client = match ChatGPT::new(key) {
                Ok(client) => client,
                Err(e) => {
                    println!("Error creating ChatGPT client: {:?}", e);
                    return;
                }
            };
            let future = gpt_client.send_message_streaming(prompt);
            let mut stream = match future.await {
                Ok(stream) => stream,
                Err(e) => {
                    println!("Error sending message to ChatGPT: {:?}", e);
                    return;
                }
            };
            let mut result = String::new();
            if let Err(why) = msg.channel_id.say(&ctx.http, "processing...").await {
                log::error!("Error sending message: {:?}", why);
            }
            while let Some(chunk) = stream.next().await {
                match chunk {
                    ResponseChunk::Content {
                        delta,
                        response_index: _,
                    } => {
                        print!("{}", delta);
                        stdout().lock().flush().unwrap();
                        result.push_str(&delta);
                    }
                    _ => {}
                }
            }
        }
    }
}

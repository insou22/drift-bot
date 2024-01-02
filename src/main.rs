mod driving_sports;

use std::collections::HashSet;
use std::pin::pin;
use std::time::Duration;

use dotenv::dotenv;
use regex::Regex;
use serenity::all::{ChannelId, GuildId};
use serenity::async_trait;
use serenity::builder::{CreateMessage, CreateAllowedMentions};
use serenity::framework::standard::StandardFramework;
use serenity::futures::StreamExt;
use serenity::prelude::*;
use tokio::time::sleep;

#[allow(unused)]
const ZAC_BOTS_ID: GuildId = GuildId::new(1191667614431314040);
const GENERAL_ID: ChannelId = ChannelId::new(1191667614431314043);

const DRIVING_SPORTS_MESSAGE: &str = "@everyone New DrivingSports event:";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let new_event_message_pattern = Regex::new(&format!(r"^{DRIVING_SPORTS_MESSAGE} \*\*(.*)\*\*$")).unwrap();

        loop {
            println!("Checking for previously broadcasted events...");
            let mut broadcasted_events = vec![];
            
            let mut messages = pin!(GENERAL_ID.messages_iter(ctx.http()));
            while let Some(message) = messages.next().await {
                if let Ok(message) = message {
                    if message.author.id == ctx.cache.current_user().id {
                        if let Some(captures) = new_event_message_pattern.captures(&message.content) {
                            broadcasted_events.push(captures.get(1).unwrap().as_str().to_string());
                        }
                    }
                }
            }

            println!("Have previously broadcasted:");
            for event in &broadcasted_events {
                println!("* {event}");
            }

            loop {
                println!("Checking current events...");

                let current_events: HashSet<_> = HashSet::from_iter(driving_sports::get_driving_sports_events().await);

                println!("Found current events:");
                for event in &current_events {
                    println!("* {event}");
                }

                let new_events = &current_events - &HashSet::from_iter(broadcasted_events.clone());

                println!("New events:");
                for event in &new_events {
                    println!("* {event}");
                }

                for event in &new_events {
                    println!("Notifying for new event: {event}");

                    let message = CreateMessage::new()
                        .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                        .content(format!("{DRIVING_SPORTS_MESSAGE} **{event}**"));

                    GENERAL_ID.send_message(ctx.http(), message)
                        .await
                        .expect("Failed to send message");
                }

                sleep(Duration::from_secs(60)).await;

                if !new_events.is_empty() {
                    println!("Refreshing previously broadcasted events...");
                    break;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN")
        .expect("Missing DISCORD_TOKEN");

    let framework = StandardFramework::new();
    let intents = GatewayIntents::all();
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

mod adc;
pub mod common;
mod driving_sports;

use adc::Adc;
use common::EventFinder;
use driving_sports::DrivingSports;

use std::{pin::pin, time::Duration, env};

use dotenv::dotenv;
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    framework::standard::StandardFramework,
    futures::StreamExt,
    prelude::*,
};
use tokio::time::sleep;

#[allow(unused)]
const ZAC_BOTS_ID: GuildId = GuildId::new(1191667614431314040);
#[allow(unused)]
const TEST_CHANNEL_ID: ChannelId = ChannelId::new(1191767432730267769);
const GENERAL_ID: ChannelId = ChannelId::new(1191667614431314043);

struct DiscordBot;

#[async_trait]
impl EventHandler for DiscordBot {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let mut event_finders: Vec<Box<dyn EventFinder>> =
            vec![Box::new(DrivingSports::new()), Box::new(Adc::new())];

        loop {
            println!("Checking for previously broadcasted events...");

            let mut messages = pin!(GENERAL_ID.messages_iter(ctx.http()));
            while let Some(message) = messages.next().await {
                if let Ok(message) = message {
                    if message.author.id == ctx.cache.current_user().id {
                        for event_finder in &mut event_finders {
                            event_finder.previous_broadcast(&message);
                        }
                    }
                }
            }

            loop {
                println!("Checking current events...");

                let mut broadcast_occured = false;

                for event_finder in &event_finders {
                    let new_broadcasts = event_finder.new_broadcasts().await;

                    match new_broadcasts {
                        Ok(new_broadcasts) => {
                            for broadcast in new_broadcasts {
                                broadcast_occured = true;

                                if let Err(err) =
                                    GENERAL_ID.send_message(ctx.http(), broadcast).await
                                {
                                    eprintln!("Error sending discord message: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error discovering events: {err}");
                        }
                    }
                }

                sleep(Duration::from_secs(60)).await;

                if broadcast_occured {
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

    if env::var("RUST_BACKTRACE").is_err() {
        env::set_var("RUST_BACKTRACE", "1");
    }

    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");

    let framework = StandardFramework::new();
    let intents = GatewayIntents::all();
    let mut client = Client::builder(token, intents)
        .event_handler(DiscordBot)
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {why:?}");
    }
}

use reqwest::{Client, IntoUrl};
use serenity::{async_trait, all::Message, builder::CreateMessage};

#[async_trait]
pub trait EventFinder: Send + Sync {
    fn previous_broadcast(&mut self, message: &Message);
    async fn new_broadcasts(&self) -> Vec<CreateMessage>;
}

pub async fn fetch_page_text(client: &Client, url: impl IntoUrl) -> String {
    client.get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}
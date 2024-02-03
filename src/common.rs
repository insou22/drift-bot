use std::{future::Future, pin::Pin};

use reqwest::{Client, IntoUrl};
use serenity::{all::Message, builder::CreateMessage};

pub trait EventFinder: Send + Sync {
    fn previous_broadcast(&mut self, message: &Message);
    fn new_broadcasts<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Vec<CreateMessage>> + Send + 'a>>
    where
        Self: 'a;
}

pub async fn fetch_page_text(client: &Client, url: impl IntoUrl + Send) -> String {
    client.get(url).send().await.unwrap().text().await.unwrap()
}

pub async fn fetch_sitemap_urls(client: &Client, sitemap_url: impl IntoUrl + Send) -> Vec<String> {
    let sitemap_text = fetch_page_text(client, sitemap_url).await;
    let sitemap_xml = roxmltree::Document::parse(&sitemap_text).unwrap();

    let urlset = sitemap_xml
        .descendants()
        .find(|node| node.has_tag_name("urlset"))
        .unwrap();

    let urls = urlset
        .children()
        .filter_map(|node| node.children().find(|node| node.has_tag_name("loc")))
        .filter_map(|loc| loc.text())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    urls
}

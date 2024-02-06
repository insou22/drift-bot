use std::{future::Future, pin::Pin};

use anyhow::{Context, Result};
use reqwest::Client;
use serenity::{all::Message, builder::CreateMessage};

pub trait EventFinder: Send + Sync {
    fn previous_broadcast(&mut self, message: &Message);
    fn new_broadcasts<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CreateMessage>>> + Send + 'a>>
    where
        Self: 'a;
}

pub async fn fetch_page_text(client: &Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?
        .text()
        .await
        .with_context(|| format!("Failed to read text from {url}"))
}

pub async fn fetch_sitemap_urls(client: &Client, sitemap_url: &str) -> Result<Vec<String>> {
    let sitemap_text = fetch_page_text(client, sitemap_url).await?;
    let sitemap_xml =
        roxmltree::Document::parse(&sitemap_text).context("Failed to parse sitemap as XML")?;

    let urlset = sitemap_xml
        .descendants()
        .find(|node| node.has_tag_name("urlset"))
        .context("Sitemap is missing urlset")?;

    let urls = urlset
        .children()
        .filter_map(|node| node.children().find(|node| node.has_tag_name("loc")))
        .filter_map(|loc| loc.text())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    Ok(urls)
}

use std::{collections::HashSet, future::Future, pin::Pin};

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::ClientBuilder;
use scraper::{Html, Selector};
use serenity::{
    all::Message,
    builder::{CreateAllowedMentions, CreateEmbed, CreateMessage},
};

use crate::common::{fetch_page_text, fetch_sitemap_urls, EventFinder};

const ADC_COM: &str = "https://www.australiandriftclub.com.au/";
const EVENT_SITEMAP_PATH: &str = "wp-sitemap-posts-mep_events-1.xml";
const TITLE_SELECTOR: &str = ".mep-default-title > h2";
const BANNER_SELECTOR: &str = ".mep-event-thumbnail > img";
const ADC_MESSAGE: &str = "@everyone New ADC event:";

static MESSAGE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!(r"^{ADC_MESSAGE} \*\*(.*)\*\*$")).unwrap());

pub struct Adc {
    previous_titles: HashSet<String>,
}

impl Adc {
    pub fn new() -> Self {
        Self {
            previous_titles: HashSet::new(),
        }
    }
}

impl EventFinder for Adc {
    fn previous_broadcast(&mut self, message: &Message) {
        if let Some(captures) = MESSAGE_PATTERN.captures(&message.content) {
            self.previous_titles
                .insert(captures.get(1).unwrap().as_str().to_string());
        }
    }

    fn new_broadcasts<'a>(&'a self) -> Pin<Box<dyn Future<Output = Vec<CreateMessage>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        Box::pin(async move {
            get_adc_events()
                .await
                .into_iter()
                .filter(|event| !self.previous_titles.contains(&event.title))
                .map(|event| {
                    let mut message = CreateMessage::new()
                        .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                        .content(format!("{ADC_MESSAGE} **{}**", event.title));

                    if let Some(banner_url) = event.banner_url {
                        message = message.add_embed(
                            CreateEmbed::new()
                                .title(&event.title)
                                .url(&event.url)
                                .image(banner_url),
                        );
                    }

                    message
                })
                .collect()
        })
    }
}

#[derive(Debug)]
pub struct AdcEvent {
    pub url: String,
    pub title: String,
    pub banner_url: Option<String>,
}

pub async fn get_adc_events() -> Vec<AdcEvent> {
    let client = ClientBuilder::new().build().unwrap();

    let urls = fetch_sitemap_urls(&client, format!("{ADC_COM}{EVENT_SITEMAP_PATH}")).await;

    let mut events = Vec::new();

    for url in urls {
        let page_text = fetch_page_text(&client, &url).await;
        let page = Html::parse_document(&page_text);
        let title_selector = Selector::parse(TITLE_SELECTOR).unwrap();
        let banner_selector = Selector::parse(BANNER_SELECTOR).unwrap();

        let title = page.select(&title_selector).next().unwrap().inner_html();

        let banner_url = page
            .select(&banner_selector)
            .next()
            .unwrap()
            .attr("data-src")
            .map(ToString::to_string);

        events.push(AdcEvent {
            url,
            title,
            banner_url,
        });
    }

    events
}

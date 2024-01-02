use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::ClientBuilder;
use scraper::{Selector, Html};
use serenity::{async_trait, all::Message, builder::{CreateMessage, CreateAllowedMentions, CreateEmbed}};

use crate::common::{EventFinder, fetch_page_text};

const ADC_COM: &str = "https://www.australiandriftclub.com.au/";
const EVENT_SITEMAP_PATH: &str = "wp-sitemap-posts-mep_events-1.xml";
const TITLE_SELECTOR: &str = ".mep-default-title > h2";
const BANNER_SELECTOR: &str = ".mep-event-thumbnail > img";
const ADC_MESSAGE: &str = "@everyone New ADC event:";

static MESSAGE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(&format!(r"^{ADC_MESSAGE} \*\*(.*)\*\*$")).unwrap());

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

#[async_trait]
impl EventFinder for Adc {
    fn previous_broadcast(&mut self, message: &Message) {
        if let Some(captures) = MESSAGE_PATTERN.captures(&message.content) {
            self.previous_titles.insert(captures.get(1).unwrap().as_str().to_string());
        }
    }

    async fn new_broadcasts(&self) -> Vec<CreateMessage> {
        get_adc_events().await.into_iter()
            .filter(|event| !self.previous_titles.contains(&event.title))
            .map(|event| CreateMessage::new()
                .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                .content(format!("{ADC_MESSAGE} **{}**", event.title))
                .add_embed(
                    CreateEmbed::new()
                        .title(&event.title)
                        .url(&event.url)
                        .image(&event.banner_url))
            )
            .collect()
    }
}

#[derive(Debug)]
pub struct AdcEvent {
    pub url: String,
    pub title: String,
    pub banner_url: String,
}

pub async fn get_adc_events() -> Vec<AdcEvent> {
    let client = ClientBuilder::new()
        .build()
        .unwrap();

    let sitemap_text = fetch_page_text(&client, format!("{ADC_COM}{EVENT_SITEMAP_PATH}")).await;
    let sitemap_xml = roxmltree::Document::parse(&sitemap_text).unwrap();

    let urlset = sitemap_xml.descendants()
        .find(|node| node.has_tag_name("urlset"))
        .unwrap();

    let urls = urlset.children()
        .filter_map(|node| node.children().find(|node| node.has_tag_name("loc")))
        .filter_map(|loc| loc.text())
        .collect::<Vec<_>>();

    let mut events = Vec::new();

    for url in urls {
        let page_text = fetch_page_text(&client, url).await;
        let page = Html::parse_document(&page_text);
        let title_selector = Selector::parse(TITLE_SELECTOR).unwrap();
        let banner_selector = Selector::parse(BANNER_SELECTOR).unwrap();

        let title = page.select(&title_selector)
            .next()
            .unwrap()
            .inner_html();

        let banner_url = page.select(&banner_selector)
            .next()
            .unwrap()
            .attr("data-src")
            .unwrap();

        events.push(AdcEvent {
            url: url.to_string(),
            title,
            banner_url: banner_url.to_string(),
        });
    }

    println!("{events:#?}");

    events
}
use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::ClientBuilder;
use scraper::{Selector, Html};
use serenity::{all::Message, async_trait, builder::{CreateMessage, CreateAllowedMentions}};

use crate::common::{EventFinder, fetch_page_text};

const DRIVING_SPORTS_COM: &str = "https://www.drivingsports.com.au/";
const MENU_ITEM_SELECTOR: &str = ".menu-item-object-page > a";
const TITLE_SELECTOR: &str = "h1.entry-title.page-title";

const NON_EVENT_ITEMS: &[&str] = &[
    "home",
    "members",
    "scrutineering",
    "events",
    "contact",
];

const DRIVING_SPORTS_MESSAGE: &str = "@everyone New DrivingSports event:";
static MESSAGE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(&format!(r"^{DRIVING_SPORTS_MESSAGE} \*\*(.*)\*\*$")).unwrap());

pub struct DrivingSports {
    previous_events: HashSet<String>,
}

impl DrivingSports {
    pub fn new() -> Self {
        Self {
            previous_events: HashSet::new(),
        }
    }
}

#[async_trait]
impl EventFinder for DrivingSports {
    fn previous_broadcast(&mut self, message: &Message) {
        if let Some(captures) = MESSAGE_PATTERN.captures(&message.content) {
            self.previous_events.insert(captures.get(1).unwrap().as_str().to_string());
        }
    }

    async fn new_broadcasts(&self) -> Vec<CreateMessage> {
        let current_events: HashSet<_> = HashSet::from_iter(get_driving_sports_events().await);
        let new_events = &current_events - &self.previous_events;

        new_events.into_iter()
            .map(|event| CreateMessage::new()
                .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                .content(format!("{DRIVING_SPORTS_MESSAGE} **{event}**")))
            .collect()
    }
}

pub async fn get_driving_sports_events() -> Vec<String> {
    let client = ClientBuilder::new()
        .build()
        .unwrap();

    let response = fetch_page_text(&client, DRIVING_SPORTS_COM).await;

    let other_events = {
        // stupid `Html` isn't Send, so don't want to hold across .await
        let parsed_homepage = Html::parse_document(&response);
        let selector = Selector::parse(MENU_ITEM_SELECTOR).unwrap();
    
        parsed_homepage.select(&selector)
            .filter_map(|item| item.attr("href"))
            .map(|item| item.trim_start_matches(DRIVING_SPORTS_COM).trim_end_matches('/'))
            .filter(|item| !item.is_empty())
            .filter(|item| !NON_EVENT_ITEMS.contains(item))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };

    let mut unparsed_events = vec![response];
    for other_event in other_events {
        let response = fetch_page_text(&client, format!("{DRIVING_SPORTS_COM}{other_event}/")).await;

        unparsed_events.push(response);
    }

    let parsed_events = unparsed_events.into_iter()
        .map(|event| Html::parse_document(&event))
        .collect::<Vec<_>>();

    let title_selector = Selector::parse(TITLE_SELECTOR).unwrap();

    let titles = parsed_events.iter()
        .filter_map(|page| page.select(&title_selector).next())
        .map(|title| title.inner_html())
        .collect::<Vec<_>>();

    println!("Events: {titles:?}");

    titles
}
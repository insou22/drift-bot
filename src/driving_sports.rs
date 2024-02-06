use std::{collections::HashSet, future::Future, pin::Pin};

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::ClientBuilder;
use scraper::{Html, Selector};
use serenity::{
    all::Message,
    builder::{CreateAllowedMentions, CreateMessage},
};

use crate::common::{fetch_page_text, fetch_sitemap_urls, EventFinder};

const DRIVING_SPORTS_COM: &str = "https://www.drivingsports.com.au/";
const POSTS_SITEMAP_PATH: &str = "wp-sitemap-posts-page-1.xml";
const TITLE_SELECTOR: &str = "h1.entry-title.page-title";

const NON_EVENT_ITEMS: &[&str] = &[
    "home",
    "members",
    "scrutineering",
    "events",
    "contact",
    "track-day",
    "terms-conditions",
    "events-2",
];

const DRIVING_SPORTS_MESSAGE: &str = "@everyone New DrivingSports event:";
// Backwards compatibility on URL formatting
static MESSAGE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^{DRIVING_SPORTS_MESSAGE} \[?\*\*(.*)\*\*(\]\(.*\))?$"
    ))
    .expect("Is a valid regex")
});

pub struct DrivingSports {
    previous_titles: HashSet<String>,
}

impl DrivingSports {
    pub fn new() -> Self {
        Self {
            previous_titles: HashSet::new(),
        }
    }
}

impl EventFinder for DrivingSports {
    fn previous_broadcast(&mut self, message: &Message) {
        if let Some(captures) = MESSAGE_PATTERN.captures(&message.content) {
            self.previous_titles.insert(
                captures
                    .get(1)
                    .expect("Capture group 1 is present in regex")
                    .as_str()
                    .to_string(),
            );
        }
    }

    fn new_broadcasts<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CreateMessage>>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        Box::pin(async move {
            Ok(get_driving_sports_events()
                .await?
                .into_iter()
                .filter(|event| !self.previous_titles.contains(&event.title))
                .map(|event| {
                    CreateMessage::new()
                        .allowed_mentions(CreateAllowedMentions::new().everyone(true))
                        .content(format!(
                            "{DRIVING_SPORTS_MESSAGE} [**{}**]({})",
                            event.title, event.url
                        ))
                })
                .collect())
        })
    }
}

#[derive(Debug)]
pub struct DrivingSportsEvent {
    pub url: String,
    pub title: String,
}

pub async fn get_driving_sports_events() -> Result<Vec<DrivingSportsEvent>> {
    let client = ClientBuilder::new()
        .build()
        .expect("System TLS must be present");

    let urls = fetch_sitemap_urls(
        &client,
        &format!("{DRIVING_SPORTS_COM}{POSTS_SITEMAP_PATH}"),
    )
    .await?;
    let mut urls = urls.into_iter().peekable();

    let mut events = Vec::new();

    // Lots of garbage and old events before DRIVING_SPORTS_COM
    while let Some(url) = urls.peek() {
        if url == DRIVING_SPORTS_COM {
            break;
        }

        urls.next();
    }

    let title_selector =
        Selector::parse(TITLE_SELECTOR).expect("TITLE_SELECTOR is known to be valid");

    for url in urls {
        let page_name = url
            .trim_start_matches(DRIVING_SPORTS_COM)
            .trim_end_matches('/');

        if NON_EVENT_ITEMS.contains(&page_name) || page_name.contains("form") {
            continue;
        }

        let page_contents = fetch_page_text(&client, &url).await?;
        let page = Html::parse_document(&page_contents);

        match page.select(&title_selector).next() {
            Some(title) => {
                let title = title.inner_html();

                events.push(DrivingSportsEvent { url, title });
    
            }
            None => {
                eprintln!("Failed to load DrivingSports page title {url} (ignoring and continuing)");
            }
        }
    }

    println!("Current DrivingSports events:");
    for DrivingSportsEvent { url, title } in &events {
        println!("- {title} ({url})");
    }
    println!();

    Ok(events)
}

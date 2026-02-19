use anyhow::Result;
use colored::*;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    #[serde(default)]
    item: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: Option<String>,
    link: Option<String>,
    #[serde(rename = "pubDate")]
    pub_date: Option<String>,
}

pub async fn check_news() -> Result<()> {
    println!("{}", ":: Checking Arch Linux News...".blue().bold());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15)) // Increased from 5 to 15 seconds
        .user_agent("ax/1.0") // Better compatibility
        .build()?;

    let content = client
        .get("https://archlinux.org/feeds/news/")
        .send()
        .await?
        .text()
        .await?;

    let rss: Rss = quick_xml::de::from_str(&content)?;

    // Show last 3 items
    for item in rss.channel.item.iter().take(3) {
        let title = item.title.as_deref().unwrap_or("No Title");
        let link = item.link.as_deref().unwrap_or("");
        let pub_date = item.pub_date.as_deref().unwrap_or("");

        println!("{} {} ({})", "->".yellow(), title.bold(), pub_date.cyan());
        println!("   {}", link.dimmed());
    }
    println!();

    Ok(())
}

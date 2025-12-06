use anyhow::Result;
use colored::*;
use rss::Channel;
use std::time::Duration;

pub async fn check_news() -> Result<()> {
    println!("{}", ":: Checking Arch Linux News...".blue().bold());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let content = client.get("https://archlinux.org/feeds/news/")
        .send()
        .await?
        .bytes()
        .await?;

    let channel = Channel::read_from(&content[..])?;

    // Show last 3 items
    for item in channel.items().iter().take(3) {
        let title = item.title().unwrap_or("No Title");
        let link = item.link().unwrap_or("");
        let pub_date = item.pub_date().unwrap_or("");

        println!("{} {} ({})", "->".yellow(), title.bold(), pub_date.cyan());
        println!("   {}", link.dimmed());
    }
    println!();

    Ok(())
}

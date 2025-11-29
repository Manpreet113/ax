use serde::Deserialize;
use anyhow::{Result, Context};

#[derive(Deserialize, Debug)]
struct AurResponse {
    results: Vec<AurPackage>
}

#[derive(Deserialize, Debug)]
pub struct AurPackage {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    // #[serde(rename = "Maintainer")]
    // pub maintainer: Option<String>,
    #[serde(rename = "NumVotes")]
    pub num_votes: i32,
    // #[serde(rename = "Popularity")]
    // pub popularity: f64,
}

pub async fn search(query: &str) -> Result<Vec<AurPackage>> {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg[]={}", query);
    let client =  reqwest::Client::new();
    let resp = client.get(&url).send().await.context("Failed to contact AUR")?;
    let json: AurResponse = resp.json().await.context("Failed to parse AUR response")?;

    Ok(json.results)
}



use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct AurPackage {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Maintainer")]
    pub _maintainer: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "NumVotes")]
    pub num_votes: Option<i32>,
}

#[derive(Deserialize, Debug)]
struct AurResponse {
    results: Vec<AurPackage>,
}

pub async fn get_info(packages: &[String]) -> Result<Vec<AurPackage>> {
    if packages.is_empty() {
        return Ok(vec![]);
    }

    let client = reqwest::Client::new();
    let url = "https://aur.archlinux.org/rpc/?v=5&type=info";

    let params: Vec<(&str, &String)> = packages.iter().map(|p| ("arg[]", p)).collect();

    let resp = client.get(url)
        .query(&params)
        .send()
        .await?;

    let json: AurResponse = resp.json().await?;
    Ok(json.results)
}

pub async fn search(query: &str) -> Result<Vec<AurPackage>> {
    let client = reqwest::Client::new();
    let url = "https://aur.archlinux.org/rpc/?v=5&type=search";

    let resp = client.get(url)
        .query(&[("arg", query)])
        .send()
        .await?;

    let json: AurResponse = resp.json().await?;
    Ok(json.results)
}
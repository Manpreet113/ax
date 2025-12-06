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
}

#[derive(Deserialize, Debug)]
struct AurResponse {
    results: Vec<AurPackage>,
}

pub async fn get_info(packages: &[String]) -> Result<Vec<AurPackage>> {
    if packages.is_empty() {
        return Ok(vec![]);
    }

    // TODO: Chunk requests if list > 100
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
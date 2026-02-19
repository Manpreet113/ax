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

async fn make_request<T: serde::Serialize + ?Sized>(
    url: &str,
    params: &T,
) -> Result<Vec<AurPackage>> {
    let client = reqwest::Client::new();
    let mut retries = 3;
    let mut delay = 1;

    loop {
        let resp = client.get(url).query(params).send().await?;

        if resp.status().as_u16() == 429 {
            if retries > 0 {
                eprintln!("!! AUR Rate limit exceeded. Retrying in {}s...", delay);
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                retries -= 1;
                delay *= 2;
                continue;
            } else {
                anyhow::bail!("AUR Rate limit exceeded. Giving up.");
            }
        }

        if !resp.status().is_success() {
            anyhow::bail!("AUR API Request failed: {}", resp.status());
        }

        let json: AurResponse = resp.json().await?;
        return Ok(json.results);
    }
}

pub async fn get_info(packages: &[String]) -> Result<Vec<AurPackage>> {
    if packages.is_empty() {
        return Ok(vec![]);
    }

    let url = "https://aur.archlinux.org/rpc/?v=5&type=info";
    let params: Vec<(&str, &String)> = packages.iter().map(|p| ("arg[]", p)).collect();

    make_request(url, &params).await
}

pub async fn search(query: &str) -> Result<Vec<AurPackage>> {
    let url = "https://aur.archlinux.org/rpc/?v=5&type=search";
    let params = [("arg", query)];

    make_request(url, &params).await
}

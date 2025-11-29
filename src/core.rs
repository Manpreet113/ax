#[derive(Debug)]
pub struct Package {
    pub repo: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}
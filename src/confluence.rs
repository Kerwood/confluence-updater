use crate::content_payload::{ContentPayload, LabelRoot};
use crate::error::ConUpdaterError;
use crate::page::{Config, Page};
use comrak::{markdown_to_html, ComrakOptions};
use futures::future::try_join_all;
use hex;
use reqwest::{RequestBuilder, Response};
use serde::Deserialize;
use serde_yaml;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::BufReader;
use std::process;
use std::str;

pub struct Confluence {
    user: String,
    secret: String,
    fqdn: String,
    pages: Vec<Page>,
}

#[derive(Deserialize)]
pub struct VersionRoot {
    pub version: Version,
}

#[derive(Deserialize)]
pub struct Version {
    pub number: u64,
}

impl Confluence {
    pub fn new(user: String, secret: String, fqdn: String, config_path: String) -> Self {
        let pages = Self::get_page_config(&config_path).unwrap_or_else(|x| {
            println!("Could not read config file.\n[Error: {}]", x.to_string());
            process::exit(1)
        });
        Self {
            user,
            secret,
            fqdn,
            pages,
        }
    }

    pub async fn update_pages(&self) -> Result<(), ConUpdaterError> {
        let mut futures = Vec::<_>::new();

        for page in self.pages.iter() {
            futures.push(self.update_reqwest(page));
        }

        try_join_all(futures).await?;
        Ok(())
    }

    fn get_page_config(config_path: &str) -> Result<Vec<Page>, ConUpdaterError> {
        let file = fs::File::open(config_path)?;
        let reader = BufReader::new(file);
        let config: Config = serde_yaml::from_reader(reader)?;
        Ok(config.content)
    }

    async fn get_version(&self, page_id: &str) -> Result<u64, ConUpdaterError> {
        let req = reqwest::Client::new().get(format!(
            "https://{}/wiki/rest/api/content/{}",
            &self.fqdn, page_id
        ));

        let res = self
            .reqwest_handler(req)
            .await?
            .json::<VersionRoot>()
            .await?;

        Ok(res.version.number + 1)
    }

    fn render_markdown(&self, file_path: &str) -> Result<String, ConUpdaterError> {
        let file_content: String = fs::read_to_string(file_path)?;
        let html = markdown_to_html(&file_content, &ComrakOptions::default())
            .replace("\n</code>", "</code>"); // Atlassian adds an extra line to code blocks. Removing it here.
        Ok(html)
    }

    async fn update_reqwest(&self, page: &Page) -> Result<(), ConUpdaterError> {
        let html = self.render_markdown(&page.file_path)?;
        let html_sha = self.hash_str(&html);
        let current_sha = self
            .get_current_content_sha(&page.page_id)
            .await?
            .unwrap_or_default();

        if html_sha == current_sha {
            println!(
                "[ID:{}][SHA:{}] :: Skipped {} - {}",
                page.page_id, html_sha, page.title, page.file_path
            );
            return Ok(());
        }

        let version = self.get_version(&page.page_id).await?;

        let mut labels: Vec<String> = page.labels.iter().cloned().collect();

        labels.push(format!("sha:{}", html_sha));

        let payload = ContentPayload::new(&page, version, &labels, &html);

        let url = format!(
            "https://{}/wiki/rest/api/content/{}",
            &self.fqdn, &page.page_id
        );

        let req = reqwest::Client::new().put(url).json(&payload);
        self.reqwest_handler(req).await?;

        println!(
            "[ID:{}][SHA:{}] :: Updated {} [v.{}] - {}",
            page.page_id, html_sha, page.title, version, page.file_path
        );

        Ok(())
    }

    async fn get_current_content_sha(
        &self,
        page_id: &str,
    ) -> Result<Option<String>, ConUpdaterError> {
        let req = reqwest::Client::new().get(format!(
            "https://{}/wiki/rest/api/content/{}/label",
            &self.fqdn, page_id
        ));

        let labels = self.reqwest_handler(req).await?.json::<LabelRoot>().await?;

        let current_sha: String = labels
            .results
            .iter()
            .filter(|x| x.name.starts_with("sha:"))
            .map(|x| x.name.split("sha:").collect::<String>())
            .collect();

        if current_sha.is_empty() {
            return Ok(None);
        }

        Ok(Some(current_sha))
    }

    fn hash_str(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(&result[0..4])
    }

    async fn reqwest_handler(&self, request: RequestBuilder) -> Result<Response, ConUpdaterError> {
        let response = request
            .basic_auth(&self.user, Some(&self.secret))
            .send()
            .await?
            .error_for_status()?;
        Ok(response)
    }
}

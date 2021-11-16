use comrak::{markdown_to_html, ComrakOptions};
use futures::future::try_join_all;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::error::Error;
use std::fs;
use std::io;
use std::io::BufReader;
use std::process;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload<'a> {
    pub title: &'a str,
    #[serde(rename = "type")]
    pub type_field: &'a str,
    pub status: &'a str,
    pub version: Version,
    pub body: Body<'a>,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Body<'a> {
    #[serde(borrow)]
    pub storage: Storage<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Storage<'a> {
    pub value: String,
    pub representation: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub labels: Vec<Label>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Pages {
    content: Vec<Page>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    file_path: String,
    title: String,
    page_id: String,
    content_type: String,
    labels: Vec<String>,
}

pub struct Confluence {
    user: String,
    secret: String,
    fqdn: String,
    pages: Pages,
}

impl Confluence {
    pub fn new(user: String, secret: String, fqdn: String, config_path: String) -> Self {
        let pages = Self::get_config(&config_path).unwrap_or_else(|x| {
            println!("Could not read config file.\n[Error: {}]", x);
            process::exit(1)
        });
        Self {
            user,
            secret,
            fqdn,
            pages: pages,
        }
    }

    fn get_config(config_path: &str) -> Result<Pages, Box<dyn Error>> {
        let file = fs::File::open(config_path)?;
        let reader = BufReader::new(file);
        let pages = serde_yaml::from_reader(reader)?;
        Ok(pages)
    }

    async fn get_version(&self, page_id: &str) -> Result<u64, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "https://{}/wiki/rest/api/content/{}",
                &self.fqdn, page_id
            ))
            .basic_auth(&self.user, Some(&self.secret))
            .send()
            .await?;

        if !response.status().is_success() {
            println!("[Reqwest Error] {}", response.status());
            process::exit(2)
        }

        let json = response.json::<serde_json::Value>().await?;

        let version = match json.get("version") {
            Some(x) => x.get("number").unwrap().as_u64().unwrap(),
            None => {
                println!("Could not get the content version [ID: {}]", page_id);
                process::exit(3)
            }
        };
        Ok(version + 1)
    }

    fn get_markdown(&self, file_path: &str) -> Result<String, io::Error> {
        let file_content: String = fs::read_to_string(file_path)?;
        let result = markdown_to_html(&file_content, &ComrakOptions::default()).replace("\n", "");
        Ok(result)
    }

    async fn make_reqwest(&self, page: &Page) -> Result<(), Box<dyn Error>> {
        let version = self.get_version(&page.page_id).await?;
        let html = self.get_markdown(&page.file_path)?;

        let labels = page
            .labels
            .clone()
            .into_iter()
            .map(|x| Label { name: x })
            .collect();

        let payload = UpdatePayload {
            title: &page.title,
            type_field: &page.content_type,
            status: "current",
            version: Version { number: version },
            body: Body {
                storage: Storage {
                    value: html,
                    representation: "storage",
                },
            },
            metadata: Metadata { labels: labels },
        };

        let client = reqwest::Client::new();
        client
            .put(format!(
                "https://{}/wiki/rest/api/content/{}",
                &self.fqdn, &page.page_id
            ))
            .basic_auth(&self.user, Some(&self.secret))
            .json(&payload)
            .send()
            .await?;

        println!(
            "[ID:{}] :: Updated {} - {}",
            page.page_id, page.title, page.file_path
        );

        Ok(())
    }

    pub async fn update_pages(&self) -> Result<(), Box<dyn Error>> {
        let mut futures = Vec::<_>::new();

        for page in self.pages.content.iter() {
            futures.push(self.make_reqwest(page));
        }

        try_join_all(futures).await.unwrap_or_else(|x| {
            println!("[Error] {}", x);
            process::exit(4)
        });
        Ok(())
    }
}

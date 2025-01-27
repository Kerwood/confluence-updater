use super::ConfluencePage;
use crate::error::{Error, Result};
use reqwest::{ClientBuilder, Response};
use serde::Deserialize;
use tracing::{debug, info, instrument, Level};
use url::Url;

#[derive(Deserialize, Debug)]
pub struct PageResponse {
    pub id: String,
    pub version: Version,
    pub labels: Option<Labels>,
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub number: u64,
}

#[derive(Deserialize, Debug)]
pub struct Labels {
    pub results: Vec<LabelResult>,
}

#[derive(Deserialize, Debug)]
pub struct LabelResult {
    pub name: String,
}

#[derive(Debug)]
pub struct ConfluenceClient {
    client: reqwest::Client,
    base_url: String,
    user: String,
    secret: String,
}

impl ConfluenceClient {
    #[instrument(skip_all, err(Debug, level = Level::DEBUG))]
    pub fn new<T: ConfluenceClientTrait>(config: &T) -> Result<Self> {
        let client = ClientBuilder::new().build()?;
        let base_url = config.fqdn()?.to_string();

        debug!(%base_url, "creating confluence client");

        Ok(Self {
            client,
            base_url,
            user: config.username(),
            secret: config.secret(),
        })
    }

    #[instrument(skip(self), ret(level = Level::TRACE), fields(base_url = self.base_url, path = path, method = method.to_string()))]
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        debug!("created request");
        self.client
            .request(method, format!("{}/{}", self.base_url, path))
            .basic_auth(self.user.to_string(), Some(self.secret.to_string()))
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn get(&self, path: &str) -> Result<Response> {
        self.request(reqwest::Method::GET, path)
            .send()
            .await?
            .error_for_status()
            .map_err(Error::from)
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn put<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<Response> {
        self.request(reqwest::Method::PUT, path)
            .json(body)
            .send()
            .await?
            .error_for_status()
            .map_err(Error::from)
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn get_page_version(&self, page_id: &str) -> Result<u64> {
        let response = self
            .get(format!("/wiki/api/v2/pages/{}", page_id).as_ref())
            .await?
            .json::<PageResponse>()
            .await?;
        let version = response.version.number;

        Ok(version)
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn get_page_sha(&self, page_id: &str) -> Result<Option<String>> {
        let path = format!("/wiki/api/v2/pages/{}?include-labels=true", page_id);
        let response = self.get(&path).await?.json::<PageResponse>().await?;

        let sha_label: String = match response.labels {
            Some(labels) => labels
                .results
                .iter()
                .filter(|x| x.name.starts_with("sha:"))
                .map(|x| x.name.split("sha:").collect::<String>())
                .collect(),
            None => String::new(),
        };

        match sha_label.is_empty() {
            true => Ok(None),
            false => Ok(Some(sha_label)),
        }
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn update_confluence_page<T: UpdatePageTrait>(
        &self,
        page: &T,
    ) -> Result<Option<reqwest::Response>> {
        let page_id = page.id();
        let version = self.get_page_version(&page_id).await? + 1;

        if let Some(sha) = self.get_page_sha(&page_id).await? {
            if sha == page.sha() {
                info!("no changes to page, skipping.");
                return Ok(None);
            }
        }

        let mut labels = page.labels().clone();
        labels.push(format!("sha:{}", page.sha()));

        let confluence_page =
            ConfluencePage::new(&page.title(), version, &labels, &page.html_content());

        // Below URL is for Confluence APIv1 because v2 does not support updating labels yet.
        let path = format!("/wiki/rest/api/content/{}", &page_id);
        let response = self.put(&path, &confluence_page).await?;

        info!("successfully updated page.");

        Ok(Some(response))
    }
}

pub trait ConfluenceClientTrait {
    fn fqdn(&self) -> Result<Url>;
    fn username(&self) -> String;
    fn secret(&self) -> String;
}

pub trait UpdatePageTrait {
    fn title(&self) -> String;
    fn id(&self) -> String;
    fn labels(&self) -> Vec<String>;
    fn html_content(&self) -> String;
    fn sha(&self) -> String;
}

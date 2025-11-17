use super::restriction::Restriction;
use super::ConfluencePage;
use crate::config::Page;
use crate::error::{Error, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::Form,
    ClientBuilder, Response,
};
use serde::Deserialize;
use tracing::{debug, error, info, instrument, Level};

#[derive(Deserialize, Debug)]
pub struct PageResponse {
    pub version: Version,
    pub labels: Option<Labels>,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Deserialize, Debug)]
pub struct Links {
    base: String,
    webui: String,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub account_id: String,
    pub email: String,
}

#[derive(Debug)]
pub struct ConfluenceClient {
    client: reqwest::Client,
    base_url: String,
    user: String,
    secret: String,
}

impl ConfluenceClient {
    #[instrument(skip_all, name = "confluence_client::new" err(Debug, level = Level::DEBUG))]
    pub fn new(fqdn: &str, user: &str, secret: &str) -> Result<Self> {
        let client = ClientBuilder::new().build()?;
        let base_url = fqdn.to_string();

        if !base_url.starts_with("https://") {
            let error = Error::HttpsProtocolSchemeMissing(base_url);
            error!(%error);
            return Err(error);
        }

        debug!(%base_url, "creating confluence client");

        Ok(Self {
            client,
            base_url,
            user: user.to_string(),
            secret: secret.to_string(),
        })
    }

    #[instrument(skip(self), ret(level = Level::TRACE), fields(base_url = self.base_url, path = path, method = method.to_string()))]
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        debug!("created request");
        self.client
            .request(method, format!("{}{}", self.base_url, path))
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
    async fn put_multiform(
        &self,
        path: &str,
        header_map: Option<HeaderMap<HeaderValue>>,
        form: Form,
    ) -> Result<Response> {
        let mut req = self.request(reqwest::Method::PUT, path).multipart(form);

        if let Some(headers) = header_map {
            req = req.headers(headers);
        }

        req.send().await?.error_for_status().map_err(Error::from)
    }
    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn get_page_version(&self, page_id: &str) -> Result<u64> {
        let response = self
            .get(format!("/wiki/api/v2/pages/{page_id}").as_ref())
            .await?
            .json::<PageResponse>()
            .await?;
        let version = response.version.number;

        Ok(version)
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn get_page_link(&self, page_id: &str) -> Result<String> {
        let response = self
            .get(format!("/wiki/api/v2/pages/{page_id}").as_ref())
            .await?
            .json::<PageResponse>()
            .await?;
        let base = response.links.base;
        let webui = response.links.webui;

        Ok(format!("{base}{webui}"))
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn get_current_user(&self) -> Result<User> {
        let response = self
            .get("/wiki/rest/api/user/current")
            .await?
            .json::<User>()
            .await?;

        Ok(response)
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn upload_attachment(&self, page_id: &str, file_path: &str) -> Result<()> {
        let path = format!("/wiki/rest/api/content/{page_id}/child/attachment");

        let mut header_map = HeaderMap::new();
        header_map.insert("X-Atlassian-Token", HeaderValue::from_static("nocheck"));

        let form = Form::new()
            .text("minorEdit", "true")
            .file("file", file_path)
            .await?;

        self.put_multiform(&path, Some(header_map), form).await?;

        Ok(())
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    async fn get_page_sha(&self, page_id: &str) -> Result<Option<String>> {
        let path = format!("/wiki/api/v2/pages/{page_id}?include-labels=true");
        let response = self.get(&path).await?.json::<PageResponse>().await?;

        let sha_label: String = match response.labels {
            Some(labels) => labels
                .results
                .iter()
                .filter(|x| x.name.starts_with("page-sha/"))
                .map(|x| x.name.split("page-sha/").collect::<String>())
                .collect(),
            None => String::new(),
        };

        match sha_label.is_empty() {
            true => Ok(None),
            false => Ok(Some(sha_label)),
        }
    }

    async fn remove_page_restriction(&self, page_id: &str) -> Result<()> {
        let body = Restriction::no_restrictions();
        let path = format!("/wiki/rest/api/content/{page_id}/restriction");
        self.put(&path, &body).await?;
        Ok(())
    }

    async fn set_page_read_only(&self, page_id: &str, account_id: &str) -> Result<()> {
        let body = Restriction::read_only(account_id);
        let path = format!("/wiki/rest/api/content/{page_id}/restriction");
        self.put(&path, &body).await?;
        Ok(())
    }

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn update_confluence_page(&self, page: &Page) -> Result<Option<reqwest::Response>> {
        let version = self.get_page_version(&page.page_id).await? + 1;

        if let Some(sha) = self.get_page_sha(&page.page_id).await? {
            if sha == page.page_sha {
                info!("no changes to page, skipping.");
                return Ok(None);
            }
        }

        let user_label = self
            .get_current_user()
            .await?
            .email
            .split_once("@")
            .ok_or(Error::CurrentUserEmailMissing)?
            .0
            .replace(".", "-");

        let labels = vec![
            format!("page-sha/{}", page.page_sha),
            format!("pa-token/{}", user_label),
        ];

        for image_path in &page.html.attachment_paths {
            info!("uploading attachment [{}]", &image_path);
            self.upload_attachment(&page.page_id, image_path)
                .await
                .inspect_err(|error| error!(image=image_path, %error))?;
        }

        let confluence_page = ConfluencePage::new(page, version).add_labels(labels);

        // Below URL is for Confluence APIv1 because v2 does not support updating labels yet.
        let path = format!("/wiki/rest/api/content/{}", &page.page_id);
        let response = self.put(&path, &confluence_page).await?;

        info!("successfully updated page.");

        if page.read_only == Some(true) {
            let account_id = self.get_current_user().await?.account_id;
            self.set_page_read_only(&page.page_id, &account_id).await?;
            debug!("set 'view only' for anyone else than current user");
        } else if page.read_only == Some(false) {
            self.remove_page_restriction(&page.page_id).await?;
            debug!("removing all page restrictions for users.");
        }

        Ok(Some(response))
    }
}

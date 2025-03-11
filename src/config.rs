use crate::confluence;
use crate::error::{Error, Result};
use crate::render_markdown::HtmlPage;
use crate::CommandArgs;
use derive_more::Debug;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tracing::{instrument, span, Level};

// ###################################################### //
//                     Config Struct                      //
// ###################################################### //

#[derive(Debug)]
pub struct Config {
    pub user: String,
    #[debug("\"<redacted>\"")]
    pub secret: String,
    pub fqdn: String,
    pub pages: Vec<Page>,
}

#[derive(Debug)]
pub struct Page {
    pub file_path: String,
    pub page_id: String,
    pub title: String,
    pub labels: Vec<String>,
    pub read_only: Option<bool>,
    #[allow(dead_code)]
    pub superscript_header: Option<String>,
    pub html: HtmlPage,
    pub page_sha: String,
}

impl Page {
    #[instrument(skip_all, ret(level = Level::TRACE), err(Display))]
    async fn try_from_async(page_config: PageConfig) -> Result<Self> {
        let html = HtmlPage::new(&page_config).await?;

        let title = match (&page_config.override_title, &html.page_header) {
            (Some(override_title), _) => override_title,
            (None, Some(page_title)) => page_title,
            (None, None) => return Err(Error::PageHeaderMissing),
        };

        let labels = match &page_config.labels {
            Some(labels) => labels.to_owned(),
            None => vec![],
        };

        let page_sha = page_config.calculate_sha()?;

        let page = Self {
            file_path: page_config.file_path,
            page_id: page_config.page_id,
            title: title.to_string(),
            labels,
            read_only: page_config.read_only,
            superscript_header: page_config.superscript_header,
            html,
            page_sha,
        };

        Ok(page)
    }
}

// ###################################################### //
//                  ConfigFile Struct                     //
// ###################################################### //

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pages: Vec<PageConfig>,
    read_only: Option<bool>,
    superscript_header: Option<String>,
}

impl ConfigFile {
    #[instrument(skip_all, ret(level = Level::TRACE), err(Display))]
    pub fn new(path: &str) -> Result<ConfigFile> {
        if !Path::new(path).is_file() {
            return Err(Error::InvalidFilePath(path.to_string()));
        }
        let file = fs::read_to_string(path)?;
        let yaml: ConfigFile = serde_yml::from_str(&file)?;
        Ok(yaml)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageConfig {
    pub file_path: String,
    pub page_id: String,
    pub override_title: Option<String>,
    pub labels: Option<Vec<String>>,
    pub read_only: Option<bool>,
    pub superscript_header: Option<String>,
}

impl PageConfig {
    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    fn calculate_sha(&self) -> Result<String> {
        if !Path::new(&self.file_path).is_file() {
            return Err(Error::InvalidFilePath(self.file_path.to_string()));
        }

        let mut content = std::fs::read_to_string(&self.file_path)?;
        content.push_str(self.override_title.as_ref().unwrap_or(&"".to_string()));
        content.push_str(self.superscript_header.as_ref().unwrap_or(&"".to_string()));

        if let Some(vec) = &self.labels {
            content.push_str(&vec.join(""));
        }

        let mut hasher = Sha256::new();
        hasher.update(content);

        let hash = hasher.finalize();
        let sha = hex::encode(&hash[0..4]);
        Ok(sha)
    }
}

// ###################################################### //
//             TryFrom CommandArgs -> Config              //
// ###################################################### //

impl Config {
    #[instrument(skip_all, ret(level = Level::TRACE))]
    pub async fn try_from_async(args: CommandArgs) -> Result<Self> {
        let config_file = ConfigFile::new(&args.config_path)?;

        let mut pages: Vec<Page> = vec![];

        for mut page_config in config_file.pages {
            let span = span!(
                Level::INFO,
                "page",
                id = page_config.page_id,
                path = page_config.file_path
            );

            let _enter = span.enter();

            // Overwrite superscript_header if it's set globally and not explicitly on the page.
            if config_file.superscript_header.is_some() && page_config.superscript_header.is_none()
            {
                page_config.superscript_header = config_file.superscript_header.clone();
            }

            // If there are not labes on the page config, create an empty vec.
            if page_config.labels.is_none() {
                page_config.labels = Some(vec![]);
            }

            // Add global cli labels to the page config and filter our invalid labels.
            if let Some(ref mut vec) = page_config.labels {
                vec.extend(args.labels.iter().cloned());
                *vec = confluence::filter_valid_labels(vec);
            }

            // Set default read_only to true or overwrite read_only if it's set globally and not explicitly on the page.
            page_config.read_only = match (config_file.read_only, page_config.read_only) {
                (None, None) => Some(false),
                (_, Some(page)) => Some(page),
                (Some(config), None) => Some(config),
            };

            let page = Page::try_from_async(page_config).await?;
            pages.push(page);
        }

        let config = Self {
            user: args.user,
            secret: args.secret,
            fqdn: args.fqdn,
            pages,
        };

        Ok(config)
    }
}

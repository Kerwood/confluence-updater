use crate::confluence::{ConfluenceClientTrait, UpdatePageTrait};
use crate::error::{Error, Result};
use crate::markdown;
use crate::CommandArgs;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fs;
use tracing::{debug, error, instrument, Level};
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub user: String,
    pub secret: String,
    pub fqdn: String,
    pub config_path: String,
    pub pages: Vec<PageConfig>,
}

#[derive(Deserialize, Debug)]
pub struct PageConfigRoot {
    pub pages: Vec<PageConfig>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageConfig {
    pub file_path: String,
    pub override_title: Option<String>,
    pub page_id: String,
    pub labels: Vec<String>,
    #[serde(skip)]
    html: RefCell<Option<String>>,
    #[serde(skip)]
    title: RefCell<Option<String>>,
    #[serde(skip)]
    page_sha: RefCell<Option<String>>,
}

impl PageConfig {
    fn log_error<E: std::error::Error>(error: E) -> E {
        debug!(?error);
        error!(%error);
        error
    }
}

impl UpdatePageTrait for PageConfig {
    #[instrument(skip_all, ret(level = Level::TRACE), fields(id = self.page_id,))]
    fn title(&self) -> String {
        if let Some(title) = self.title.borrow().as_ref() {
            return title.to_string();
        }

        if let Some(title) = &self.override_title {
            return title.to_string();
        }

        match markdown::get_page_title(&self.file_path) {
            Ok(title) => {
                *self.title.borrow_mut() = Some(title.to_string());
                title
            }
            Err(error) => {
                Self::log_error(error);
                std::process::exit(1);
            }
        }
    }

    #[instrument(skip_all, ret(level = Level::TRACE), fields(id = self.page_id,))]
    fn id(&self) -> String {
        self.page_id.to_owned()
    }

    #[instrument(skip_all, ret(level = Level::TRACE), fields(id = self.page_id,))]
    fn labels(&self) -> Vec<String> {
        self.labels.to_owned()
    }

    #[instrument(skip_all, ret(level = Level::TRACE), fields(id = self.page_id,))]
    fn html_content(&self) -> String {
        if let Some(html) = self.html.borrow().as_ref() {
            return html.to_string();
        }

        match markdown::render_markdown_file(&self.file_path) {
            Ok(html) => {
                *self.html.borrow_mut() = Some(html.to_string());
                html
            }
            Err(error) => {
                Self::log_error(error);
                std::process::exit(1);
            }
        }
    }

    #[instrument(skip_all, ret(level = Level::TRACE), fields(id = self.page_id,))]
    fn sha(&self) -> String {
        if let Some(sha) = self.page_sha.borrow().as_ref() {
            return sha.to_string();
        }

        let mut content = match std::fs::read_to_string(&self.file_path) {
            Ok(content) => content,
            Err(error) => {
                Self::log_error(error);
                std::process::exit(1);
            }
        };

        content.push_str(&self.title());

        let mut hasher = Sha256::new();
        hasher.update(content);

        let hash = hasher.finalize();
        let sha = hex::encode(&hash[0..4]);
        *self.page_sha.borrow_mut() = Some(sha.to_string());
        sha
    }
}

impl ConfluenceClientTrait for Config {
    fn fqdn(&self) -> Result<url::Url> {
        let mut fqdn = self.fqdn.to_owned();
        if !(fqdn.starts_with("https://") || fqdn.starts_with("http://")) {
            fqdn = format!("https://{}", fqdn);
        }
        Url::parse(fqdn.as_ref()).map_err(Error::from)
    }

    fn username(&self) -> String {
        self.user.to_owned()
    }

    fn secret(&self) -> String {
        self.secret.to_owned()
    }
}

impl TryFrom<CommandArgs> for Config {
    type Error = Error;

    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    fn try_from(args: CommandArgs) -> Result<Self, Self::Error> {
        let config_file = fs::read_to_string(&args.config_path)?;
        let mut config: PageConfigRoot = serde_yml::from_str(&config_file)?;

        for page in config.pages.iter_mut() {
            page.labels.extend(args.labels.to_owned());
        }

        let config = Self {
            user: args.user,
            secret: args.secret,
            fqdn: args.fqdn,
            config_path: args.config_path,
            pages: config.pages,
        };

        Ok(config)
    }
}

// use confluence_updater::*;
mod confluence;
use confluence::Confluence;
mod content_payload;
mod error;
mod page;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "confluence-updater",
    about = "Update content in Confluence Cloud",
    author = "Patrick Kerwood <patrick@kerwood.dk>"
)]
enum Opt {
    #[structopt(about = "Update a confluence pages")]
    Update {
        #[structopt(short, long, env = "CU_USER", help = "Confluence user to login with")]
        user: String,

        #[structopt(
            short,
            long,
            env = "CU_SECRET",
            help = "The token/secret to use. https://id.atlassian.com/manage-profile/security/api-tokens"
        )]
        secret: String,

        #[structopt(
            long,
            default_value = "your-domain.atlassian.net",
            env = "CU_FQDN",
            help = "The fully qualified domain name of your Atlassian Cloud."
        )]
        fqdn: String,

        #[structopt(
            short,
            long,
            default_value = "./confluence-config.yaml",
            env = "CU_CONFIG_PATH",
            help = "The path to the config file."
        )]
        config_path: String,

        #[structopt(
            name = "label",
            short,
            long = "label",
            help = "Add a label to all updating pages. Can be used multiple times."
        )]
        labels: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), String> {
    match Opt::from_args() {
        Opt::Update {
            user,
            secret,
            fqdn,
            config_path,
            labels,
        } => {
            let con = Confluence::new(user, secret, fqdn, config_path, labels);
            con.update_pages().await?;
        }
    }
    Ok(())
}

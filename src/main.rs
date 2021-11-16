use confluence_updater::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "confluence-updater",
    about = "Update content in Confluence Cloud",
    author = "Patrick Kerwood <patrick@kerwood.dk>"
)]
enum Opt {
    #[structopt(about = "Update a page")]
    Update {
        #[structopt(short, long, env = "CU_USER", help = "Confluence user to login with")]
        user: String,

        #[structopt(
            short,
            long,
            env = "CU_SECRET",
            help = "The token/secret to use. [https://id.atlassian.com/manage-profile/security/api-tokens]"
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
    },
}

#[tokio::main]
async fn main() {
    match Opt::from_args() {
        Opt::Update {
            user,
            secret,
            fqdn,
            config_path,
        } => {
            let con = Confluence::new(user, secret, fqdn, config_path);
            con.update_pages().await.unwrap();
        }
    }
}

# Confluence Updater

[![forthebadge made-with-rust](http://ForTheBadge.com/images/badges/made-with-rust.svg)](https://www.rust-lang.org/)

If you like to keep your documentation in Git, love writing in markdown but are somehow required to deliver documentation in Confluence, look no further.

Confluence Updater will render your markdown files to HTML and upload it to a specific page in your Confluence Cloud instance. The tool will label each page with a SHA based on the page content. If the SHA hasn't changed since last update, the page will be skipped.

It's now possible to create a build pipeline that uploads your Git documentation to your Confluence Cloud. You can find a how-to on my blog [https://linuxblog.xyz/posts/confluence-updater/](https://linuxblog.xyz/posts/confluence-updater/).

Go to [https://id.atlassian.com/manage-profile/security/api-tokens](https://id.atlassian.com/manage-profile/security/api-tokens) and create an API token.

The tool looks for a `confluence-config.yaml` file in the present directory with the configuration for which markdown files to render and which page ID to update with the content. There's an example in the repo. **You will need to crate the page and get the page ID for the configuration file.**

Run below example command to start the process.

```
confluence-updater update -u your-user@example.org -s <api-token> --fqdn your-domain.atlassian.net
```

You can use environment variables instead of parameters.

```
confluence-updater 1.1.0
Update a confluence pages

USAGE:
    confluence-updater update [OPTIONS] --secret <secret> --user <user>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config-path <config-path>    The path to the config file. [env: CU_CONFIG_PATH=]  [default: ./confluence-config.yaml]
        --fqdn <fqdn>                  The fully qualified domain name of your Atlassian Cloud. [env:CU_FQDN=]  [default: your-domain.atlassian.net]
    -l, --label <label>...             Add a label to all updating pages. Can be used multiple times.
    -s, --secret <secret>              The token/secret to use. https://id.atlassian.com/manage-profile/security/api-tokens [env: CU_SECRET=]
    -u, --user <user>                  Confluence user to login with [env: CU_USER=]
```

## Example

Either set the user, secret and FQDN with environment variables or as parameters.

```
export CU_USER=you-user@example.org
export CU_SECRET=personal-access-token
export CU_FQDN=your-domain.atlassian.net
```

Create the `confluence-config.yaml` like in the [example.](https://github.com/Kerwood/confluence-updater/blob/main/confluence-config.yaml)

Run `confluence-updater`.

```
➜  ~ confluence-updater
[ID:353237642][SHA:17998344] :: Skipped Kubernetes Install Guide - ./kubernetes-install.md
[ID:353237651][SHA:58a0b222] :: Updated Grafana Install Guide [v.61] - ./grafana-install.md
```

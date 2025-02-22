# Confluence Updater

[![forthebadge made-with-rust](http://ForTheBadge.com/images/badges/made-with-rust.svg)](https://www.rust-lang.org/)

If you prefer keeping your documentation in Git, love writing in Markdown, but need to publish it in Confluence, this tool is for you.

**Confluence Updater** renders your Markdown files into HTML and uploads them to a specified page in your Confluence Cloud instance. The tool assigns each page a SHA label based on its content. If the SHA remains unchanged since the last update, the page is skipped.

With this tool, you can automate documentation uploads in a CI/CD pipeline. Check out the [setup guide on my blog](https://linuxblog.xyz/posts/confluence-updater/).

## Authentication
Confluence Cloud requires personal access tokens for authentication.

Generate an API token at: [Atlassian API Tokens.](https://id.atlassian.com/manage-profile/security/api-tokens)

The tool applies a label using the local part of the email associated with the token. For example, `patrick@kerwood.dk` will have the label `pa-token/patrick` applied. This labeling system helps track and replace tokens when necessary.

By setting the `superscriptHeader` property in the configuration, you can quickly locate Confluence pages linked to a specific repository.

## Usage
The tool searches for a `confluence-updater.yaml` ([example here](https://github.com/Kerwood/confluence-updater/blob/main/confluence-updater.yaml)) file in the current directory. This file defines which Markdown files to render and their corresponding Confluence page IDs.

**Note:** You must manually create the Confluence page and obtain its ID beforehand.

## Basic Usage
Run the following command:

```sh
confluence-updater -u your-user@example.org -s <api-token> --fqdn https://your-domain.atlassian.net
```

Alternatively, use environment variables:

```sh
export CU_USER=your-user@example.org
export CU_SECRET=your-api-token
export CU_FQDN=https://your-domain.atlassian.net
```

```sh
$ confluence-updater
INFO Successfully updated page. id="728383651" title="Kubernetes Install Guide" path="./kubernetes-install.md" sha="6b8b051c"
INFO No changes to page, skipping. id="729133252" title="Grafana Install Guide [v.61]" path="./grafana-install.md" sha="fa3d0cdd"
```

### Command-Line Options
```sh
Usage: confluence-updater [OPTIONS] --user <USER> --secret <SECRET>

Options:
  -u, --user <USER>                Confluence login user [env: CU_USER=]
  -s, --secret <SECRET>            API token [env: CU_SECRET=]
      --fqdn <FQDN>                Atlassian Cloud domain [env: CU_FQDN=]
  -c, --config-path <CONFIG_PATH>  Config file path [default: ./confluence-updater.yaml]
  -l, --label <label>              Add labels to pages (repeatable)
      --log-level <LOG_LEVEL>      Log level [default: info] [trace, debug, info, warn, error]
  -h, --help                       Show help
  -V, --version                    Show version
```

## Features
### Content Update
Confluence Updater generates a SHA hash of the content and stores it on the Confluence page as a label.
On each run, it checks for changes using the SHA and updates the page only if modifications are detected.

The following elements are included in the SHA, meaning any changes to them will trigger a page update:
- Markdown file content.
- Override title.
- Superscript header.
- Labels.

### Link Replacement
Convert relative Markdown file links to Confluence page links using `pid:<page-id>`:

```md
[Some link text](./other/file.md "pid:5234523")
```

### Read-Only
Restrict editing to the token owner by setting `readOnly` at the root or page level:

```yaml
readOnly: true
pages:
  - filePath: ./README.md
    pageId: 228184928
    readOnly: false
```

![restrictions](./images/restrictions.png)

### Image Uploads
Markdown image links are automatically uploaded as attachments and embedded in the Confluence page.
You can control image alignment by specifying `align-left`, `align-right`, or `align-center` in the `alt` text.

```md
![align-center](./images/bender.png)
```

### Superscript Header
Add a small superscript header with Markdown support at the top of each page by setting `superscriptHeader` at the root or page level:

```yaml
superscriptHeader: This page is sourced from [kerwood/confluence-updater](https://github.com/Kerwood/confluence-updater)
pages:
  - filePath: ./README.md
    pageId: 228184928
    superscriptHeader: Some other superscript header
```

![superscriptheader](./images/superscriptheader.png)
### Code Blocks
Markdown code blocks are converted into Confluence `CodeBlock` macros with syntax highlighting:

````md
```rust
let x = "hello-world";
```
````

![code-example](./images/code-example.png)

## Example Workflow

1. Set environment variables or use command-line parameters.
2. Create a `confluence-updater.yaml` file ([example here](https://github.com/Kerwood/confluence-updater/blob/main/confluence-updater.yaml)).
3. Run the updater:

```sh
confluence-updater
```

Example output:
```sh
INFO Successfully updated page. id="728383651" title="Kubernetes Install Guide" path="./kubernetes-install.md" sha="6b8b051c"
INFO No changes to page, skipping. id="729133252" title="Grafana Install Guide [v.61]" path="./grafana-install.md" sha="fa3d0cdd"
```

## GitHub Action Support

A GitHub Action is available: [Confluence Updater Action](https://github.com/Kerwood/confluence-updater-action).

```yaml
- name: Confluence Updater
  uses: kerwood/confluence-updater-action@v1
  with:
    fqdn: your-domain.atlassian.net
    user: ${{ secrets.USER }}
    api_token: ${{ secrets.API_TOKEN }}
```

## Release Notes

### 2.1.1
- Changed the `pa-token:xxx` and `page-sha:xxx` labels to use supported characters. (`pa-token/xxx`, `page-sha/xxx`)
- Added a regex check to all labels. If labes are not valid they will be skipped and a warning will be emitted.
- Added better error logs if FQDN is missing `https://` protocol scheme.
- Confluence updater now uploads page images before it updates the page.

### 2.1.0
- Added support for superscript page header. See [Superscript Header](#superscript-header)
- Added support for images. See [Image Uploads](#image-uploads).
- Added page restriction support. See [Read Only](#read-only).
- Added link replacement. See [Link Replacement](#link-replacement).
- Personal Access Token label now uses the local part of the email. See [Authentication](#authentication).
- Renamed `sha` label to `page-sha`.
- Removed `url` crate dependency.
- Protocol schema (`https://`) is now mandatory in the FQDN parameter.
- Fixed [#3](https://github.com/Kerwood/confluence-updater/issues/3): indentation issue in code blocks.

### 2.0.0 (Breaking Changes)
- Extracts the top `h1` header from Markdown as the Confluence page title.
- Added `overrideTitle` property to specify a custom title.
- Switched to `CodeBlock` macro for syntax highlighting.
- Replaced `content` property with `pages`.
- Removed `contentType`.
- Switched from `serde_yaml` to `serde_yml`.
- Replaced `structopt` with `clap`.
- Switched from `openssl` to `rusttls`.
- Implemented structured logging using `tracing`.

---

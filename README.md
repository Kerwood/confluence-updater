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
Usage: confluence-updater [OPTIONS] --user <USER> --secret <SECRET> --fqdn <FQDN>

Options:
  -u, --user <USER>                Confluence user to login with [env: CU_USER=]
  -s, --secret <SECRET>            The token/secret to use. https://id.atlassian.com/manage-profile/security/api-tokens [env: CU_SECRET=]
      --fqdn <FQDN>                The fully qualified domain name of your Atlassian Cloud. [env: CU_FQDN=]
  -c, --config-path <CONFIG_PATH>  The path to the config file. [env: CU_CONFIG_PATH=] [default: ./confluence-updater.yaml]
  -l, --label <label>              Add a label to all updating pages. Can be used multiple times.
      --log-level <LOG_LEVEL>      Log Level. [env: CU_LOG_LEVEL=] [default: info] [possible values: trace, debug, info, warn, error]
  -h, --help                       Print help
  -V, --version                    Print version
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
- Read Only boolean

### Link Replacement
Convert relative Markdown file links to Confluence page links using `pid:<page-id>`:

```md
[Some link text](./other/file.md "pid:5234523")
```

### Read-Only
If the `readOnly` property is **not** set at the global and page level configuration, Confluence Updater will not
modify any existing page restrictions. This allows page restrictions to be set manually within Confluence without
being overwritten by Confluence Updater.

- Setting `readOnly: true` grants read-only access to everyone except the access token owner, who will retain write permissions.
- Setting `readOnly: false` removes all user restrictions, effectively making the page editable by anyone in the Confluence space.

Changing this setting will affect the `sha` label and trigger a page update.

#### Example
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

**NOTE:** The Confluence API is a bit unstable when it comes to uploading attachments.
This is the reason that images are not uploaded asynchronously, the API simply can't handle it.

Some times you will recieve below error for no apparent reason.
```
ERROR image="images/superscriptheader.png" error=HTTP request, HTTP status server error (500 Internal Server Error)
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

### v2.2.0
- Added support for removing page restrictions by setting `readOnly: false`. See [Read Only](#read-only)
- Added support for preserving existing page restrictions by omitting the `readOnly` property entirely. See [Read Only](#read-only)
- Added validation for `fqdn`, `user`, and `secret` arguments to ensure values are **not** quoted, with clear error messages for invalid input.
- Changed log level env name from `LOG_LEVEL` to `CU_LOG_LEVEL`.

### v2.1.1
- Changed the `pa-token:xxx` and `page-sha:xxx` labels to use supported characters. (`pa-token/xxx`, `page-sha/xxx`)
- Added a regex check to all labels. If labes are not valid they will be skipped and a warning will be emitted.
- Added better error logs if FQDN is missing `https://` protocol scheme.
- Confluence updater now uploads page images before it updates the page.

### v2.1.0
- Added support for superscript page header. See [Superscript Header](#superscript-header)
- Added support for images. See [Image Uploads](#image-uploads).
- Added page restriction support. See [Read Only](#read-only).
- Added link replacement. See [Link Replacement](#link-replacement).
- Personal Access Token label now uses the local part of the email. See [Authentication](#authentication).
- Renamed `sha` label to `page-sha`.
- Removed `url` crate dependency.
- Protocol schema (`https://`) is now mandatory in the FQDN parameter.
- Fixed [#3](https://github.com/Kerwood/confluence-updater/issues/3): indentation issue in code blocks.

### v2.0.0 (Breaking Changes)
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

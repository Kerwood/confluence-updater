# Confluence Updater

If you like to keep your documentation in Git, love writing in markdown but are somehow required to deliver documentation in Confluence, look no further.

This tool will render your markdown files to html and upload it to a page in your Confluence Cloud instance via the API. I haven't tested it on a self hosted instance so I'm don't know if that will work.

It's now possible to create a build pipeline that creates the documentation for you in Confluence.

Go to [https://id.atlassian.com/manage-profile/security/api-tokens](https://id.atlassian.com/manage-profile/security/api-tokens) and create an API token.

The tool looks for a `confluence-config.yaml` file in the present directory with the configuration for which markdown files to render and which page ID to update with the content. There's an example in the repo. **You will need to crate the page and get the page ID for the configuration file.**

Run below example command to start the process.

```
confluence-updater update -u your-user@example.org -s <api-token> --fqdn your-domain.atlassian.net
```

You can use environment variables instead of parameters.

```
confluence-updater-update 0.1.0
Update a page

USAGE:
    confluence-updater update [OPTIONS] --secret <secret> --user <user>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config-path <config-path>    The path to the config file. [env: CU_CONFIG_PATH=]  [default: ./confluence-
                                       config.yaml]
        --fqdn <fqdn>                  The fully qualified domain name of your Atlassian Cloud. [env: CU_FQDN=]
                                       [default: your-domain.atlassian.net]
    -s, --secret <secret>              The token/secret to use. [https://id.atlassian.com/manage-profile/security/api-
                                       tokens] [env:
                                       CU_SECRET=ZndNxSlfTOAfuyEFv8Z61863]
    -u, --user <user>                  Confluence user to login with [env: CU_USER=]
```

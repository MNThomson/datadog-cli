# datadog-cli

[<img alt="github" src="https://img.shields.io/badge/github-MNThomson/datadog--cli-bc3f48?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/MNThomson/datadog-cli)
[<img alt="crates.io" src="https://img.shields.io/crates/v/datadog-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/datadog-cli)
[<img alt="crates.io" src="https://img.shields.io/crates/d/datadog-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/datadog-cli)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/MNThomson/datadog-cli/ci.yml?branch=master&style=for-the-badge&logo=githubactions&logoColor=white" height="20">](https://github.com/MNThomson/datadog-cli/actions?query=branch%3Amaster)


CLI tool for querying Datadog logs (since the UI sucks at loading them).

```console
$ cargo install datadog-cli

$ export DD_API_KEY=...
$ export DD_APP_KEY=...

$ datadog logs 'status:error'
[2026-01-05 12:34:56] ERROR | Connection timeout to database
[2026-01-05 12:34:12] ERROR | Failed to process request

$ datadog logs 'service:myapp status:error "timeout"' --from now-30d --to now-2h --limit 5
[2026-01-05 12:34:56] ERROR | Connection timeout to database
```

Or use the [1Password CLI](https://developer.1password.com/docs/cli/) to inject secrets:

```bash
alias datadog='op run --no-masking --env-file=<(echo -e "DD_API_KEY=op://private/Datadog API/api_key\nDD_APP_KEY=op://private/Datadog API/app_key") -- ~/.local/share/cargo/bin/datadog'
```

#### License

<sup>
Licensed under <a href="LICENSE">AGPL-3.0</a>
</sup>
<br>
<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you shall be licensed as above, without any additional terms or conditions
</sub>

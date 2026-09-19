# Portfolio site

A small static-site generator written in Rust. It reads my public GitHub repositories, keeps the ones tagged with the `portfolio` topic, and writes a static site to `dist/`, which is deployed to Cloudflare Pages.

## Use it

1. Edit the settings at the top of `src/main.rs` (GitHub username, name, contact links).
2. Write the About page in `content/about.html`.
3. Tag up to six repositories with the `portfolio` topic on GitHub.
4. `cargo run` builds the site into `dist/`. Preview it by pointing Caddy's `root` at that folder.

Set `GITHUB_TOKEN` in the environment if you hit GitHub's anonymous rate limit.

## Deploy

`.github/workflows/deploy.yml` rebuilds and deploys on every push to `main` and once a day. It needs two repository secrets: `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.
# mysite

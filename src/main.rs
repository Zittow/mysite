//! Static portfolio generator.
//!
//! Reads your public repositories from GitHub, keeps the ones tagged with
//! FEATURE_TOPIC, and writes a static site into ./dist. Run with `cargo run`.

use serde::Deserialize;
use std::{env, error::Error, fs, path::Path};

// ------------------------------------------------------------------ settings
// Edit these, then rebuild.

const GITHUB_USER: &str = "Zittow";
/// Only repositories carrying this GitHub topic are shown.
const FEATURE_TOPIC: &str = "portfolio";
const MAX_PROJECTS: usize = 3;

const SITE_NAME: &str = "Morris Gavander";
const ROLE: &str = "IT Support & Operations Technician";
const INTRO: &str = "I work with Microsoft 365, Intune, Autopilot and PowerShell. \
Below are projects of mine, pulled straight from GitHub.";
const EMAIL: &str = "MorrisGavander@Gmail.com";
const LINKEDIN_URL: &str = "https://www.linkedin.com/in/morris-gavander";

// The stylesheet and the About page text live in their own files.
const STYLE: &str = include_str!("../assets/style.css");
const ABOUT_CONTENT: &str = include_str!("../content/about.html");

// ---------------------------------------------------------------- GitHub API

#[derive(Deserialize)]
struct Repo {
    name: String,
    html_url: String,
    description: Option<String>,
    language: Option<String>,
    homepage: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    pushed_at: Option<String>,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    archived: bool,
}

fn api_get(url: &str, accept: &str) -> Result<ureq::Response, ureq::Error> {
    let mut request = ureq::get(url)
        .set("User-Agent", "portfolio-generator")
        .set("Accept", accept)
        .set("X-GitHub-Api-Version", "2022-11-28");
    // Optional: a token raises the rate limit. GitHub Actions provides one automatically.
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
    }
    request.call()
}

fn fetch_repos() -> Result<Vec<Repo>, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/users/{GITHUB_USER}/repos?per_page=100&type=owner&sort=pushed"
    );
    let repos: Vec<Repo> = api_get(&url, "application/vnd.github+json")?.into_json()?;
    Ok(repos)
}

/// Fallback for repos without a description: the first plain paragraph of the README.
fn readme_summary(repo_name: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{GITHUB_USER}/{repo_name}/readme");
    let text = api_get(&url, "application/vnd.github.raw+json")
        .ok()?
        .into_string()
        .ok()?;

    let mut paragraph: Vec<&str> = Vec::new();
    let mut in_code = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let noise = line.starts_with('#')
            || line.starts_with("![")
            || line.starts_with("[![")
            || line.starts_with('<')
            || line.starts_with('|')
            || line.starts_with("---");
        if line.is_empty() || noise {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(line);
    }

    let joined = paragraph
        .join(" ")
        .replace(|c: char| c == '*' || c == '`', "");
    if joined.is_empty() {
        None
    } else {
        Some(truncate(&joined, 220))
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut: String = s.chars().take(max_chars).collect();
    format!("{}…", cut.trim_end())
}

// ---------------------------------------------------------------- HTML output

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Wraps a page body in the shared header and footer.
fn page(title: &str, description: &str, active: &str, body: &str) -> String {
    let link = |href: &str, label: &str, key: &str| {
        let current = if key == active {
            r#" aria-current="page""#
        } else {
            ""
        };
        format!(r#"<a href="{href}"{current}>{label}</a>"#)
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{description}">
<link rel="stylesheet" href="/style.css">
</head>
<body>
<header class="site-header wrap">
<a class="brand" href="/">{name}</a>
<nav aria-label="Main">{projects}{about}</nav>
</header>
<main class="wrap">
{body}
</main>
<footer class="site-footer wrap">
<p>This site is generated in Rust from my <a href="https://github.com/{user}" target="_blank" rel="noopener noreferrer">GitHub repositories</a>.</p>
</footer>
</body>
</html>
"#,
        title = esc(title),
        description = esc(description),
        name = esc(SITE_NAME),
        projects = link("/", "Projects", "home"),
        about = link("/about", "About", "about"),
        body = body,
        user = GITHUB_USER,
    )
}

fn project_row(repo: &Repo, summary: &str) -> String {
    let language = repo.language.as_deref().unwrap_or("Various");

    let updated_html = match repo.pushed_at.as_deref().and_then(|d| d.get(..10)) {
        Some(date) => format!(
            r#"<p class="project-updated">Updated <time datetime="{date}">{date}</time></p>"#
        ),
        None => String::new(),
    };

    let topics: Vec<String> = repo
        .topics
        .iter()
        .filter(|t| t.as_str() != FEATURE_TOPIC)
        .map(|t| esc(t))
        .collect();
    let topics_html = if topics.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="project-topics">{}</p>"#, topics.join(", "))
    };

    let demo_html = match repo.homepage.as_deref() {
        Some(url) if url.starts_with("http") => {
            format!(r#"<a href="{}" target="_blank" rel="noopener noreferrer">Live demo</a>"#, esc(url))
        }
        _ => String::new(),
    };

    format!(
        r#"<li class="project">
<div class="project-facts">
<p class="project-lang">{language}</p>
{updated_html}
</div>
<div class="project-body">
<h3>{name}</h3>
<p class="project-desc">{summary}</p>
{topics_html}
<p class="project-links"><a href="{url}" target="_blank" rel="noopener noreferrer">Source code</a>{demo_html}</p>
</div>
</li>
"#,
        language = esc(language),
        updated_html = updated_html,
        name = esc(&repo.name),
        summary = esc(summary),
        topics_html = topics_html,
        url = esc(&repo.html_url),
        demo_html = demo_html,
    )
}

fn home_body(rows: &str) -> String {
    let projects = if rows.is_empty() {
        r#"<p class="empty">No projects yet. Add the “portfolio” topic to a repository on GitHub, then rebuild the site.</p>"#
            .to_string()
    } else {
        format!(r#"<ul class="projects">{rows}</ul>"#)
    };

    format!(
        r#"<section class="hero">
<h1>{name}</h1>
<p class="role">{role}</p>
<p class="lede">{intro}</p>
<p class="contact"><a href="mailto:{email}">Email</a><a href="{linkedin}" target="_blank" rel="noopener noreferrer">LinkedIn</a><a href="https://github.com/{user}" target="_blank" rel="noopener noreferrer">GitHub</a><a href="/about">About and CV</a></p>
</section>
<section id="projects" aria-labelledby="projects-heading">
<h2 id="projects-heading">Projects</h2>
{projects}
</section>"#,
        name = esc(SITE_NAME),
        role = esc(ROLE),
        intro = esc(INTRO),
        email = esc(EMAIL),
        linkedin = esc(LINKEDIN_URL),
        user = GITHUB_USER,
        projects = projects,
    )
}

// ---------------------------------------------------------------------- main

fn main() -> Result<(), Box<dyn Error>> {
    if GITHUB_USER == "YOUR_GITHUB_USERNAME" {
        return Err("Set GITHUB_USER (and your contact details) at the top of src/main.rs first".into());
    }

    let mut repos: Vec<Repo> = fetch_repos()?
        .into_iter()
        .filter(|r| !r.fork && !r.archived && r.topics.iter().any(|t| t == FEATURE_TOPIC))
        .collect();
    repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at)); // most recently pushed first
    repos.truncate(MAX_PROJECTS);

    if repos.is_empty() {
        eprintln!("Warning: no public repos tagged \"{FEATURE_TOPIC}\" found for {GITHUB_USER}.");
    }

    let rows: String = repos
        .iter()
        .map(|r| {
            let summary = r
                .description
                .clone()
                .filter(|d| !d.trim().is_empty())
                .or_else(|| readme_summary(&r.name))
                .unwrap_or_default();
            project_row(r, &summary)
        })
        .collect();

    let home = page(
        SITE_NAME,
        &format!("{SITE_NAME}, {ROLE}. Projects and CV."),
        "home",
        &home_body(&rows),
    );

    let about_body = format!(
        r#"<article class="prose">
<h1>About</h1>
{ABOUT_CONTENT}
</article>"#
    );
    let about = page(
        &format!("About | {SITE_NAME}"),
        &format!("About {SITE_NAME}: experience, skills and certifications."),
        "about",
        &about_body,
    );

    let not_found = page(
        "Page not found",
        "Page not found",
        "",
        r#"<article class="prose"><h1>Page not found</h1><p>That page doesn't exist. Go to the <a href="/">projects</a> or read the <a href="/about">about page</a>.</p></article>"#,
    );

    let out = Path::new("dist");
    fs::create_dir_all(out.join("about"))?;
    fs::write(out.join("index.html"), home)?;
    fs::write(out.join("about").join("index.html"), about)?;
    fs::write(out.join("404.html"), not_found)?;
    fs::write(out.join("style.css"), STYLE)?;

    println!("Built {} project(s) into dist/", repos.len());
    Ok(())
}

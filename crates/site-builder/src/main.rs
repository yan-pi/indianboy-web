use anyhow::{Context, Result};
use comrak::{markdown_to_html, Options};
use serde::Deserialize;
use std::{collections::HashSet, env, fs, io::Cursor, path::Path};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{styled_line_to_highlighted_html, IncludeBackground},
    parsing::SyntaxSet,
};
use time::{
    format_description, format_description::well_known::Rfc2822, Date, OffsetDateTime, Time,
    UtcOffset,
};

#[derive(Debug, Deserialize)]
struct Frontmatter {
    title: String,
    description: Option<String>,
    #[serde(rename = "publishedAt")]
    published_at: String,
    #[serde(default)]
    tags: Vec<String>,
    author: Option<String>,
    summary: Option<String>,
}

#[derive(Debug)]
struct Post {
    slug: String,
    meta: Frontmatter,
    html: String,
    toc: String,
    reading_time: usize,
}

#[derive(Debug, Deserialize)]
struct Site {
    name: String,
    site_name: String,
    url: String,
    intro: String,
    bio: String,
    location: String,
    email: String,
    #[serde(default)]
    links: Vec<Link>,
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    work: Vec<Work>,
}

#[derive(Debug, Deserialize)]
struct Link {
    label: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
    description: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Work {
    company: String,
    title: String,
    start: String,
    end: String,
    url: String,
}

fn main() -> Result<()> {
    let root = env::current_dir().context("get current directory")?;
    let output = root.join("dist");
    if output.exists() {
        fs::remove_dir_all(&output).context("remove previous dist directory")?;
    }
    fs::create_dir_all(&output)?;

    let site: Site = toml::from_str(
        &fs::read_to_string(root.join("content/site.toml")).context("read content/site.toml")?,
    )
    .context("parse content/site.toml")?;

    let mut posts = Vec::new();
    for entry in fs::read_dir(root.join("content/posts")).context("read content/posts")? {
        let entry = entry?;
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("md") {
            posts.push(parse_post(&entry.path())?);
        }
    }
    posts.sort_by(|a, b| b.meta.published_at.cmp(&a.meta.published_at));

    copy_dir(&root.join("assets"), &output.join("assets"))?;
    fs::write(
        output.join("_redirects"),
        "/blog/coding-confortable /blog/coding-comfortable 308\n/blog/use-semmantic-release /blog/use-semantic-release 308\n",
    )?;
    write_page(&output.join("index.html"), &render_home(&site, &posts))?;
    write_page(&output.join("blog.html"), &render_archive(&site, &posts))?;
    for post in &posts {
        write_page(
            &output.join(format!("blog/{}.html", post.slug)),
            &render_post(&site, post),
        )?;
    }
    fs::write(output.join("rss.xml"), render_rss(&site, &posts))?;
    fs::write(
        output.join("robots.txt"),
        format!(
            "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n",
            site.url
        ),
    )?;
    fs::write(output.join("sitemap.xml"), render_sitemap(&site, &posts))?;

    println!("built {} post(s) into {}", posts.len(), output.display());
    Ok(())
}

fn parse_post(path: &Path) -> Result<Post> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let (frontmatter, body) = source
        .strip_prefix("---\n")
        .and_then(|source| source.split_once("\n---\n"))
        .with_context(|| format!("{} must start with YAML frontmatter", path.display()))?;
    let meta: Frontmatter = serde_yaml::from_str(frontmatter)
        .with_context(|| format!("parse frontmatter in {}", path.display()))?;

    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.render.unsafe_ = true;

    let body = expand_markdown_plus(body);
    let (html, toc) = add_heading_ids(markdown_to_html(&body, &options));
    let html = highlight_code_blocks(rewrite_image_urls(html));
    let reading_time = body.split_whitespace().count().max(1).div_ceil(200);
    let slug = path
        .file_stem()
        .and_then(|name| name.to_str())
        .context("post filename is not valid UTF-8")?
        .to_owned();

    Ok(Post {
        slug,
        meta,
        html,
        toc,
        reading_time,
    })
}

fn expand_markdown_plus(source: &str) -> String {
    let expanded_covers = expand_cover_directives(source);
    let mut output = String::with_capacity(expanded_covers.len());
    let mut rest = expanded_covers.as_str();
    while let Some(start) = rest.find("[[note:") {
        output.push_str(&rest[..start]);
        let content_start = start + "[[note:".len();
        let Some(end) = rest[content_start..].find("]]") else {
            output.push_str(&rest[start..]);
            return output;
        };
        let note = rest[content_start..content_start + end].trim();
        output.push_str(
            "<span class=\"sidenote-reference\" aria-label=\"Side note\">†</span><span class=\"sidenote\" role=\"note\">",
        );
        output.push_str(note);
        output.push_str("</span>");
        rest = &rest[content_start + end + 2..];
    }
    output.push_str(rest);
    output
}

fn expand_cover_directives(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("[[cover:") {
        output.push_str(&rest[..start]);
        let content_start = start + "[[cover:".len();
        let Some(end) = rest[content_start..].find("]]") else {
            output.push_str(&rest[start..]);
            return output;
        };
        let mut fields = rest[content_start..content_start + end]
            .splitn(3, '|')
            .map(str::trim);
        let src = fields.next().unwrap_or("");
        let alt = fields.next().unwrap_or("");
        let caption = fields.next().unwrap_or("");
        output.push_str(&format!(
            r#"<figure><img src="{}" alt="{}"><figcaption>{}</figcaption></figure>"#,
            escape(src),
            escape(alt),
            escape(caption)
        ));
        rest = &rest[content_start + end + 2..];
    }
    output.push_str(rest);
    output
}

fn add_heading_ids(html: String) -> (String, String) {
    let mut output = String::with_capacity(html.len() + 128);
    let mut toc = String::new();
    let mut used = HashSet::new();
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(relative_start) = html[cursor..].find("<h") else {
            output.push_str(&html[cursor..]);
            break;
        };
        let start = cursor + relative_start;
        let Some(level) = html[start + 2..]
            .chars()
            .next()
            .and_then(|ch| ch.to_digit(10))
        else {
            output.push_str(&html[cursor..start + 2]);
            cursor = start + 2;
            continue;
        };
        let open = format!("<h{}>", level);
        let close = format!("</h{}>", level);
        if !(2..=4).contains(&level) || !html[start..].starts_with(&open) {
            output.push_str(&html[cursor..start + 2]);
            cursor = start + 2;
            continue;
        }
        let content_start = start + open.len();
        let Some(relative_end) = html[content_start..].find(&close) else {
            output.push_str(&html[cursor..]);
            break;
        };
        let content_end = content_start + relative_end;
        let text = strip_tags(&html[content_start..content_end]);
        let base = slugify(&text);
        let base = if base.is_empty() {
            "section".to_owned()
        } else {
            base
        };
        let mut id = base.clone();
        let mut suffix = 2;
        while !used.insert(id.clone()) {
            id = format!("{}-{}", base, suffix);
            suffix += 1;
        }
        output.push_str(&html[cursor..start]);
        output.push_str(&format!(r#"<h{} id="{}">"#, level, escape(&id)));
        output.push_str(&html[content_start..content_end]);
        output.push_str(&close);
        toc.push_str(&format!(
            r##"<li class="toc-level-{}"><a href="#{}">{}</a></li>"##,
            level,
            escape(&id),
            escape(&text)
        ));
        cursor = content_end + close.len();
    }
    (output, toc)
}

fn strip_tags(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_owned()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.to_lowercase().chars() {
        if character.is_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}

fn rewrite_image_urls(html: String) -> String {
    html.replace("src=\"/", "src=\"/assets/")
}

fn highlight_code_blocks(html: String) -> String {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let mut theme_source =
        Cursor::new(include_bytes!("../../../assets/editorial.tmTheme").as_slice());
    let Ok(theme) = ThemeSet::load_from_reader(&mut theme_source) else {
        return html;
    };
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    let open_prefix = "<pre><code class=\"language-";
    let close = "</code></pre>";

    while let Some(relative_start) = html[cursor..].find(open_prefix) {
        let start = cursor + relative_start;
        output.push_str(&html[cursor..start]);
        let language_start = start + open_prefix.len();
        let Some(language_offset) = html[language_start..].find("\">") else {
            output.push_str(&html[start..]);
            return output;
        };
        let language_end = language_start + language_offset;
        let language = &html[language_start..language_end];
        let code_start = language_end + 2;
        let Some(relative_end) = html[code_start..].find(close) else {
            output.push_str(&html[start..]);
            return output;
        };
        let code_end = code_start + relative_end;
        let source = unescape_html(&html[code_start..code_end]);
        let syntax = syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &theme);
        let highlighted = source
            .split_inclusive('\n')
            .map(|line| {
                highlighter
                    .highlight_line(line, &syntax_set)
                    .ok()
                    .and_then(|ranges| {
                        styled_line_to_highlighted_html(&ranges, IncludeBackground::No).ok()
                    })
                    .unwrap_or_else(|| escape(line))
            })
            .collect::<String>();
        output.push_str(&format!(
            "<pre><code class=\"language-{}\">{}</code></pre>",
            escape(language),
            highlighted
        ));
        cursor = code_end + close.len();
    }
    output.push_str(&html[cursor..]);
    output
}

fn unescape_html(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn render_home(site: &Site, posts: &[Post]) -> String {
    let recent = posts.iter().take(3).map(post_row).collect::<String>();
    let links = site
        .links
        .iter()
        .map(|link| {
            format!(
                r#"<a href="{}">{}</a>"#,
                escape(&link.url),
                escape(&link.label)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let projects = site
        .projects
        .iter()
        .map(|project| {
            format!(
                r#"<li><a href="{}">{}</a> — {}</li>"#,
                escape(&project.url),
                escape(&project.name),
                escape(&project.description)
            )
        })
        .collect::<String>();
    let work = site
        .work
        .iter()
        .map(|job| {
            format!(
                r#"<li><a href="{}">{} @ {}</a> <span>{} — {}</span></li>"#,
                escape(&job.url),
                escape(&job.title),
                escape(&job.company),
                escape(&job.start),
                escape(&job.end)
            )
        })
        .collect::<String>();
    let body = format!(
        r#"<section class="intro" id="about"><p class="eyebrow">{}</p><h1>{}</h1><p class="lead">{}</p><p>{}</p><p class="muted">location: {} · interests: systems, open source, freedom</p></section>
<section><div class="section-heading"><h2>Writing</h2><a href="/blog">all posts →</a></div><div class="post-list">{}</div></section>
<section class="home-section" id="projects"><div class="section-heading"><h2>Selected projects</h2></div><ul class="plain-list">{}</ul></section>
<section class="home-section" id="work"><div class="section-heading"><h2>Work</h2></div><ul class="plain-list work-list">{}</ul></section>
<section class="home-section" id="contact"><div class="section-heading"><h2>Contact</h2></div><p>Feel free to contact me at <a href="mailto:{}">{}</a>.</p><p>{}</p></section>"#,
        escape(&site.site_name),
        escape(&site.name),
        escape(&site.bio),
        links,
        escape(&site.location),
        recent,
        projects,
        work,
        escape(&site.email),
        escape(&site.email),
        links
    );
    page(site, &format!("{} — {}", site.name, site.site_name), &body)
}

fn render_archive(site: &Site, posts: &[Post]) -> String {
    let rows = posts.iter().map(post_row).collect::<String>();
    let filters = render_filters(posts);
    let body = format!(
        r#"<header class="page-heading"><p class="eyebrow">writing</p><h1>All posts</h1><p>Notes on engineering, Bitcoin, open source, and learning in public.</p></header>{}<div class="post-list" data-post-list>{}</div><p class="filter-empty" data-filter-empty hidden>No entries found with the selected filters.</p>"#,
        filters, rows
    );
    page(site, &format!("Writing — {}", site.site_name), &body)
}

fn render_filters(posts: &[Post]) -> String {
    let mut tags = posts
        .iter()
        .flat_map(|post| post.meta.tags.iter())
        .cloned()
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    if tags.is_empty() {
        return String::new();
    }
    let buttons = tags
        .iter()
        .map(|tag| {
            let count = posts
                .iter()
                .filter(|post| post.meta.tags.contains(tag))
                .count();
            format!(
                r#"<button type="button" data-tag="{}" aria-pressed="false">#{} <small>({})</small></button>"#,
                escape(tag),
                escape(tag),
                count
            )
        })
        .collect::<String>();
    format!(
        r#"<section class="filters" aria-labelledby="filter-heading"><div class="section-heading"><h2 id="filter-heading">Filter by topic</h2><button type="button" data-filter-toggle aria-expanded="false">show</button></div><div class="filter-controls" data-filter-controls hidden><button type="button" data-tag="" aria-pressed="true">all <small>({})</small></button>{}</div></section>"#,
        posts.len(),
        buttons
    )
}

fn render_post(site: &Site, post: &Post) -> String {
    let tags = if post.meta.tags.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="tags">{}</p>"#,
            post.meta
                .tags
                .iter()
                .map(|tag| escape(&format!("#{tag}")))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let description = post
        .meta
        .description
        .as_deref()
        .or(post.meta.summary.as_deref())
        .unwrap_or("");
    let toc = if post.toc.is_empty() {
        String::new()
    } else {
        format!(
            r#"<details class="toc"><summary>contents</summary><ol>{}</ol></details>"#,
            post.toc
        )
    };
    let body = format!(
        r#"<div class="reading-progress" data-reading-progress></div><article><header class="article-heading"><p class="eyebrow"><a href="/blog">writing</a> / {}</p><h1>{}</h1><p class="article-meta"><time datetime="{}">{}</time> · {} · {} min read</p><p class="article-description">{}</p>{}</header><nav class="article-tools" aria-label="Article tools"><button type="button" data-back>← back</button><button type="button" data-copy-url>copy link</button></nav>{}<div class="prose" id="article-content">{}</div>{}</article>"#,
        escape(&post.slug),
        escape(&post.meta.title),
        escape(&post.meta.published_at),
        escape(&post.meta.published_at),
        escape(post.meta.author.as_deref().unwrap_or("Yan Fernandes")),
        post.reading_time,
        escape(description),
        tags,
        toc,
        post.html,
        giscus()
    );
    page(
        site,
        &format!("{} — {}", post.meta.title, site.site_name),
        &body,
    )
}

fn post_row(post: &Post) -> String {
    let tags = post.meta.tags.join("|");
    format!(
        r#"<a class="post-row" href="/blog/{}" data-tags="{}"><span><strong>{}</strong><small>{}</small></span><time datetime="{}">{}</time></a>"#,
        escape(&post.slug),
        escape(&tags),
        escape(&post.meta.title),
        escape(
            post.meta
                .description
                .as_deref()
                .or(post.meta.summary.as_deref())
                .unwrap_or("")
        ),
        escape(&post.meta.published_at),
        escape(&post.meta.published_at)
    )
}

fn page(site: &Site, title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en" data-theme="light"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" href="/assets/favicon.svg"><title>{}</title><meta name="description" content="{}"><link rel="stylesheet" href="/assets/css/site.css"><link rel="alternate" type="application/rss+xml" href="/rss.xml" title="{}"><script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js" crossorigin="anonymous"></script><script defer src="/assets/js/site.js"></script></head><body><div class="site-shell"><header class="site-header"><a class="site-name" href="/">{}</a><nav aria-label="Primary navigation"><a href="/">home</a><a href="/blog">writing</a><a href="/#projects">work</a><a href="/#contact">contact</a><button type="button" data-theme-toggle aria-label="Switch theme">◐</button></nav></header><main id="main-content">{}</main><footer class="site-footer"><span>© Yan Fernandes</span><span><a href="/rss.xml">rss</a> · <a href="mailto:{}">email</a></span></footer></div></body></html>"#,
        escape(title),
        escape(&site.intro),
        escape(&site.site_name),
        escape(&site.site_name),
        body,
        escape(&site.email)
    )
}

fn giscus() -> String {
    r#"<section class="comments" aria-label="Comments"><h2>Comments</h2><script src="https://giscus.app/client.js" data-repo="yan-pi/indianboy" data-repo-id="R_kgDOPFUZCQ" data-category="Blog Comments" data-category-id="DIC_kwDOPFUZCc4C1gFm" data-mapping="pathname" data-strict="0" data-reactions-enabled="1" data-emit-metadata="0" data-input-position="top" data-theme="preferred_color_scheme" data-lang="en" crossorigin="anonymous" async></script></section>"#.to_owned()
}

fn rss_date(value: &str) -> String {
    let normalized = if value == "2025-06-31" {
        "2025-07-01"
    } else {
        value
    };
    let format = match format_description::parse_owned::<2>("[year]-[month]-[day]") {
        Ok(format) => format,
        Err(_) => return normalized.to_owned(),
    };
    let date = match Date::parse(normalized, &format) {
        Ok(date) => date,
        Err(_) => return normalized.to_owned(),
    };
    OffsetDateTime::new_in_offset(date, Time::MIDNIGHT, UtcOffset::UTC)
        .format(&Rfc2822)
        .unwrap_or_else(|_| normalized.to_owned())
}

fn render_rss(site: &Site, posts: &[Post]) -> String {
    let items = posts
        .iter()
        .map(|post| format!("<item><title><![CDATA[{}]]></title><link>{}/blog/{}</link><guid>{}/blog/{}</guid><description><![CDATA[{}]]></description><pubDate>{}</pubDate></item>", post.meta.title, site.url, post.slug, site.url, post.slug, post.meta.description.as_deref().unwrap_or(""), rss_date(&post.meta.published_at)))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0"><channel><title>{}</title><link>{}</link><description>{}</description>{}</channel></rss>"#,
        escape(&site.site_name),
        site.url,
        escape(&site.intro),
        items
    )
}

fn render_sitemap(site: &Site, posts: &[Post]) -> String {
    let mut urls = format!(
        "<url><loc>{}</loc></url><url><loc>{}/blog</loc></url>",
        site.url, site.url
    );
    for post in posts {
        urls.push_str(&format!(
            "<url><loc>{}/blog/{}</loc></url>",
            site.url, post.slug
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{}</urlset>"#,
        urls
    )
}

fn write_page(path: &Path, html: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, html).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_markdown_plus_directives() {
        let html = expand_markdown_plus(
            "before [[note: margin text]]\n\n[[cover: /a.png | Alt | Caption]]",
        );
        assert!(html.contains("class=\"sidenote\""));
        assert!(html.contains("<figure>"));
        assert!(html.contains("Caption"));
    }

    #[test]
    fn creates_unique_heading_ids_and_toc() {
        let (html, toc) = add_heading_ids("<h2>Section</h2><h2>Section</h2>".to_owned());
        assert!(html.contains("id=\"section\""));
        assert!(html.contains("id=\"section-2\""));
        assert_eq!(toc.matches("toc-level-2").count(), 2);
    }

    #[test]
    fn preserves_legacy_date_in_html_but_normalizes_rss() {
        assert_eq!(rss_date("2025-06-31"), "Tue, 01 Jul 2025 00:00:00 +0000");
    }

    #[test]
    fn highlights_language_code_blocks() {
        let html = highlight_code_blocks(
            "<pre><code class=\"language-rust\">fn main() {}</code></pre>".to_owned(),
        );
        assert!(html.contains("style=\"color:"));
    }
}

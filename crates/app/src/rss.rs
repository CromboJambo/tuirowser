//! RSS/Atom feed fetching and parsing.
//!
//! Converts feeds to clean markdown for TUI display with link passthrough support.

use anyhow::{Result, bail};
use std::io::Cursor;
use std::time::Duration;

/// Feed entry extracted from RSS or Atom
#[derive(Debug, Clone)]
pub struct FeedEntry {
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
    pub summary: Option<String>,
}

/// Parsed feed with metadata and entries
#[derive(Debug, Clone)]
pub struct ParsedFeed {
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub entries: Vec<FeedEntry>,
}

/// Fetch and parse an RSS or Atom feed from a URL
pub async fn fetch_feed(url: &str) -> Result<ParsedFeed> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        bail!("Failed to fetch feed from {}", url);
    }

    let content = response.text().await?;

    // Try RSS first, then Atom
    if let Ok(channel) = content.parse::<rss::Channel>() {
        return Ok(parse_rss(channel));
    }

    if let Ok(feed) = atom_syndication::Feed::read_from(Cursor::new(content.as_bytes())) {
        return Ok(parse_atom(feed));
    }

    bail!("Could not parse {} as RSS or Atom feed", url);
}

/// Parse an RSS channel into our internal format
fn parse_rss(channel: rss::Channel) -> ParsedFeed {
    let entries = channel.items().iter().map(|item| FeedEntry {
        title: item.title().unwrap_or("Untitled").to_string(),
        link: item.link().map(|l| l.to_string()),
        author: item.author().map(|a| a.to_string()),
        published: item.pub_date().map(|d| d.to_string()),
        summary: item.description().map(|s| s.to_string()),
    }).collect();

    ParsedFeed {
        title: channel.title().to_string(),
        description: Some(channel.description().to_string()),
        link: Some(channel.link().to_string()),
        entries,
    }
}

/// Parse an Atom feed into our internal format
fn parse_atom(feed: atom_syndication::Feed) -> ParsedFeed {
    let entries = feed.entries().iter().map(|entry| FeedEntry {
        title: entry.title().to_string(),
        link: entry.links()
            .iter()
            .find(|l| l.rel().is_empty() || l.rel() == "alternate")
            .map(|l| l.href().to_string()),
        author: entry.authors()
            .first()
            .map(|a| a.name().to_string()),
        published: entry.published()
            .or(Some(entry.updated()))
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string()),
        summary: entry.summary().map(|s| s.value.to_string()),
    }).collect();

    ParsedFeed {
        title: feed.title().to_string(),
        description: None,
        link: feed.links()
            .iter()
            .find(|l| l.rel().is_empty() || l.rel() == "alternate")
            .map(|l| l.href().to_string()),
        entries,
    }
}

/// Convert a parsed feed to markdown for TUI display
pub fn feed_to_markdown(feed: &ParsedFeed) -> String {
    let mut md = format!("# {}\n\n", feed.title);

    if let Some(desc) = &feed.description {
        md.push_str(&format!("{}\n\n---\n\n", desc));
    }

    for (i, entry) in feed.entries.iter().enumerate() {
        md.push_str(&format!("[{}] {}\n\n", i + 1, entry.title));

        if let Some(author) = &entry.author {
            md.push_str(&format!("By: {}\n\n", author));
        }

        if let Some(published) = &entry.published {
            md.push_str(&format!("Published: {}\n\n", published));
        }

        if let Some(summary) = &entry.summary {
            // Strip HTML tags from summary for cleaner display
            let clean_summary = strip_html_tags(summary);
            md.push_str(&format!("{}\n\n", clean_summary));
        }

        if let Some(link) = &entry.link {
            md.push_str(&format!("Read more: {}\n\n", link));
        }

        if i < feed.entries.len() - 1 {
            md.push_str("---\n\n");
        }
    }

    md
}

/// Strip HTML tags from text for cleaner display
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag {
            result.push(ch);
        }
    }

    // Clean up multiple spaces
    let mut cleaned = String::new();
    let mut prev_space = false;
    for ch in result.chars() {
        if ch == ' ' || ch == '\n' || ch == '\t' {
            if !prev_space {
                cleaned.push(' ');
                prev_space = true;
            }
        } else {
            cleaned.push(ch);
            prev_space = false;
        }
    }

    cleaned.trim().to_string()
}

/// Open a URL in the system's default browser
pub fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/c", "start"])
            .arg(url)
            .spawn()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        let html = "<p>Hello <b>world</b></p>";
        assert_eq!(strip_html_tags(html), "Hello world");

        let html2 = "<div><span>Nested</span> tags</div>";
        assert_eq!(strip_html_tags(html2), "Nested tags");
    }

    #[test]
    fn test_feed_to_markdown() {
        let feed = ParsedFeed {
            title: "Test Feed".to_string(),
            description: Some("A test feed".to_string()),
            link: None,
            entries: vec![
                FeedEntry {
                    title: "First Post".to_string(),
                    link: Some("http://example.com/1".to_string()),
                    author: Some("Alice".to_string()),
                    published: Some("2026-10-30 10:00".to_string()),
                    summary: Some("<p>First post content</p>".to_string()),
                },
            ],
        };

        let md = feed_to_markdown(&feed);
        assert!(md.contains("# Test Feed"));
        assert!(md.contains("[1] First Post"));
        assert!(md.contains("By: Alice"));
        assert!(md.contains("Read more: http://example.com/1"));
    }

    #[tokio::test]
    async fn test_rss_parsing() {
        let rss_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <link>http://example.com</link>
    <description>A test RSS feed</description>
    <item>
      <title>First Item</title>
      <link>http://example.com/1</link>
      <description>First item description</description>
    </item>
  </channel>
</rss>"#;

        let channel: rss::Channel = rss_xml.parse().expect("Failed to parse RSS");
        assert_eq!(channel.title(), "Test Feed");
        assert_eq!(channel.items().len(), 1);
        assert_eq!(channel.items()[0].title().unwrap(), "First Item");
    }

    #[tokio::test]
    async fn test_atom_parsing() {
        let atom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Atom Feed</title>
  <link href="http://example.com"/>
  <entry>
    <title>First Entry</title>
    <link href="http://example.com/1"/>
    <summary>First entry summary</summary>
  </entry>
</feed>"#;

        let feed = atom_syndication::Feed::read_from(Cursor::new(atom_xml.as_bytes()))
            .expect("Failed to parse Atom");
        assert_eq!(feed.title(), "Test Atom Feed");
        assert_eq!(feed.entries().len(), 1);
        assert_eq!(feed.entries()[0].title(), "First Entry");
    }
}
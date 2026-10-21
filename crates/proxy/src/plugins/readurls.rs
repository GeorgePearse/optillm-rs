//! Port of upstream `optillm/plugins/readurls_plugin.py`: every URL in the
//! query is fetched and its readable text (BeautifulSoup-style extraction,
//! here with the `scraper` HTML5 parser instead of lxml) is spliced in.

use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};
use serde_json::json;

use crate::approaches::{ApproachResult, Ctx};
use crate::py;

/// `extract_urls`.
pub fn extract_urls(text: &str) -> Vec<String> {
    let url = Regex::new(r#"https?://[^\s'"]+"#).unwrap();
    let trailing = Regex::new(r#"[,'"\)\]]+$"#).unwrap();
    url.find_iter(text)
        .map(|m| trailing.replace(m.as_str(), "").to_string())
        .collect()
}

fn in_script_or_style(node: ego_tree::NodeRef<'_, Node>) -> bool {
    node.ancestors().any(|a| {
        a.value()
            .as_element()
            .map(|e| e.name() == "script" || e.name() == "style")
            .unwrap_or(false)
    })
}

/// `element.get_text(strip=False)` with script/style removed.
fn get_text(el: ElementRef<'_>) -> String {
    el.descendants()
        .filter_map(|n| match n.value() {
            Node::Text(t) if !in_script_or_style(n) => Some(t.to_string()),
            _ => None,
        })
        .collect()
}

/// `element.get_text(strip=True)`: each string stripped, empties dropped, joined with "".
fn get_text_strip(el: ElementRef<'_>) -> String {
    el.descendants()
        .filter_map(|n| match n.value() {
            Node::Text(t) if !in_script_or_style(n) => {
                let s = py::strings::strip(t);
                (!s.is_empty()).then(|| s.to_string())
            }
            _ => None,
        })
        .collect()
}

/// The text-extraction half of `fetch_webpage_content`.
pub fn extract_text(html: &str, max_length: usize) -> String {
    let doc = Html::parse_document(html);
    let wanted = Selector::parse("h1, h2, h3, h4, h5, h6, p, table").unwrap();
    let mut elements: Vec<ElementRef> = Vec::new();
    for sel in ["article", "main", "div[role=\"main\"]", ".main-content"] {
        let s = Selector::parse(sel).unwrap();
        if let Some(content) = doc.select(&s).next() {
            elements = content.select(&wanted).collect();
            break;
        }
    }
    if elements.is_empty() {
        elements = doc.select(&wanted).collect();
    }
    let th = Selector::parse("th").unwrap();
    let tr = Selector::parse("tr").unwrap();
    let cell = Selector::parse("td, th").unwrap();
    let mut parts: Vec<String> = Vec::new();
    for el in elements {
        if el.value().name() == "table" {
            let mut table: Vec<String> = Vec::new();
            let headers: Vec<String> = el.select(&th).map(get_text_strip).collect();
            if !headers.is_empty() {
                table.push(headers.join(" | "));
            }
            for row in el.select(&tr) {
                let cells: Vec<String> = row.select(&cell).map(get_text_strip).collect();
                if !cells.is_empty() {
                    table.push(cells.join(" | "));
                }
            }
            parts.push(format!("\n{}\n", table.join("\n")));
        } else {
            parts.push(get_text(el));
        }
    }
    let text = parts.join(" ");
    let text = Regex::new(r"\s+").unwrap().replace_all(&text, " ");
    let text = py::strings::strip(&text).to_string();
    let text = Regex::new(r"\[.*?\]+")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    if py::strings::len(&text) > max_length {
        format!("{}...", py::strings::prefix(&text, max_length))
    } else {
        text
    }
}

/// `fetch_webpage_content(url)`.
pub async fn fetch_webpage_content(ctx: &Ctx, url: &str, max_length: usize) -> String {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10));
    if !ctx.config.ssl_verify {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = match builder.build() {
        Ok(c) => c,
        Err(e) => return format!("Error fetching content: {}", e),
    };
    // requests' PreparedRequest drops an empty query ("...?") and the fragment.
    let target = match url::Url::parse(url) {
        Ok(mut u) => {
            if u.query() == Some("") {
                u.set_query(None);
            }
            u.set_fragment(None);
            u.to_string()
        }
        Err(e) => return format!("Error fetching content: Invalid URL '{}': {}", url, e),
    };
    let resp = client
        .get(&target)
        .header(
            "User-Agent",
            format!(
                "optillm/{} (https://github.com/codelion/optillm)",
                crate::UPSTREAM_VERSION
            ),
        )
        .send()
        .await;
    let resp = match resp {
        Ok(r) => r,
        Err(e) => return format!("Error fetching content: {}", e),
    };
    let status = resp.status();
    if status.is_client_error() || status.is_server_error() {
        let kind = if status.is_client_error() {
            "Client"
        } else {
            "Server"
        };
        return format!(
            "Error fetching content: {} {} Error: {} for url: {}",
            status.as_u16(),
            kind,
            status.canonical_reason().unwrap_or(""),
            url
        );
    }
    match resp.text().await {
        Ok(body) => extract_text(&body, max_length),
        Err(e) => format!("Error fetching content: {}", e),
    }
}

/// `urlparse(url).netloc`.
fn netloc(url: &str) -> String {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    rest.split(['/', '?', '#']).next().unwrap_or("").to_string()
}

pub async fn run(_system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let mut modified = initial_query.to_string();
    for url in extract_urls(initial_query) {
        let content = fetch_webpage_content(ctx, &url, 100000).await;
        let domain = netloc(&url);
        modified = modified.replace(
            &url,
            &format!("{} [Content from {}: {}]", url, domain, content),
        );
    }
    Ok((json!(modified), 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_follows_bs4_rules() {
        let html = "<html><body><script>x()</script><div class=\"main-content\"><h1>Title</h1><p>Hello <b>world</b> [1]</p>\
                    <table><tr><th>A</th><th>B</th></tr><tr><td> 1 </td><td>2</td></tr></table></div></body></html>";
        // Matches bs4 + lxml (the "[1]" is removed after whitespace collapsing).
        assert_eq!(
            extract_text(html, 100000),
            "Title Hello world  A | B A | B 1 | 2"
        );
        assert_eq!(
            extract_urls("see https://a.b/c), and 'http://x.y/z'"),
            vec!["https://a.b/c", "http://x.y/z"]
        );
    }
}

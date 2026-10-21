//! Port of upstream `optillm/plugins/memory_plugin.py`.

use anyhow::anyhow;
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;
use crate::py::sklearn::{argsort, cosine, Tfidf};

const MEMORY_FILE_ENV: &str = "OPTILLM_MEMORY_FILE";

struct Memory {
    max_size: usize,
    items: Vec<String>,
    persist_path: Option<String>,
}

impl Memory {
    fn new(persist_path: Option<String>) -> Self {
        let mut m = Memory {
            max_size: 100,
            items: Vec::new(),
            persist_path,
        };
        if let Some(p) = m.persist_path.clone() {
            if let Ok(text) = std::fs::read_to_string(&p) {
                if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&text) {
                    let strings: Vec<String> = items
                        .into_iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    let start = strings.len().saturating_sub(m.max_size);
                    m.items = strings[start..].to_vec();
                }
            }
        }
        m
    }

    fn add(&mut self, item: String) {
        if self.items.len() >= self.max_size {
            self.items.remove(0);
        }
        self.items.push(item);
        if let Some(p) = &self.persist_path {
            let path = std::path::Path::new(p);
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, py::json::dumps(&json!(self.items))).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    fn get_relevant(&self, query: &str, n: usize) -> Result<Vec<String>, String> {
        if self.items.is_empty() {
            return Ok(Vec::new());
        }
        let (tfidf, rows) = Tfidf::fit_transform(&self.items)?;
        let q = tfidf.transform(query);
        let sims: Vec<f64> = rows.iter().map(|r| cosine(&q, r)).collect();
        let order = argsort(&sims);
        let start = order.len().saturating_sub(n);
        Ok(order[start..]
            .iter()
            .rev()
            .map(|&i| self.items[i].clone())
            .collect())
    }
}

/// `extract_query(text)` -> (query, context).
fn extract_query(text: &str) -> (String, String) {
    if let Some(i) = text.rfind("Query:") {
        return (
            py::strings::strip(&text[i + 6..]).to_string(),
            py::strings::strip(&text[..i]).to_string(),
        );
    }
    let splitter = fancy_regex::Regex::new(r"(?<=[.!?])\s+").unwrap();
    let stripped = py::strings::strip(text);
    let mut sentences = Vec::new();
    let mut last = 0;
    for m in splitter.find_iter(stripped).flatten() {
        sentences.push(&stripped[last..m.start()]);
        last = m.end();
    }
    sentences.push(&stripped[last..]);
    if sentences.len() > 1 {
        (
            sentences[sentences.len() - 1].to_string(),
            sentences[..sentences.len() - 1].join(" "),
        )
    } else {
        (
            "What is the main point of this text?".to_string(),
            text.to_string(),
        )
    }
}

async fn extract_key_information(
    system: &str,
    text: &str,
    query: &str,
    ctx: &Ctx,
) -> anyhow::Result<(Vec<String>, i64)> {
    #[rustfmt::skip]
    let prompt = format!(
        concat!(
            "\n",
            "'''text\n",
            "{p0}\n",
            "'''\n",
            "Copy over all context relevant to the query: {p1}\n",
            "Provide the answer in the format: <YES/NO>#<Relevant context>.\n",
            "Here are rules:\n",
            "- If you don't know how to answer the query - start your answer with NO#\n",
            "- If the text is not related to the query - start your answer with NO#\n",
            "- If you can extract relevant information - start your answer with YES#\n",
            "- If the text does not mention the person by name - start your answer with NO#\n",
            "Example answers:\n",
            "- YES#Western philosophy originated in Ancient Greece in the 6th century BCE with the pre-Socratics.\n",
            "- NO#No relevant context.\n",
        ),
        p0 = text, p1 = query,
    );
    let response = match ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", system), msg("user", &prompt)],
            "max_tokens": 1000
        }))
        .await
    {
        Ok(r) => r,
        Err(_) => return Ok((Vec::new(), 0)),
    };
    let key_info = match response.content_at(0) {
        Ok(Some(c)) => py::strings::strip(&c).to_string(),
        _ => return Ok((Vec::new(), 0)),
    };
    let mut margins = Vec::new();
    if key_info.starts_with("YES#") {
        margins.push(
            key_info
                .split_once('#')
                .map(|x| x.1.to_string())
                .unwrap_or_default(),
        );
    }
    Ok((margins, response.completion_tokens()?))
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let persist = std::env::var(MEMORY_FILE_ENV)
        .ok()
        .filter(|v| !v.is_empty());
    let mut memory = Memory::new(persist);
    let (query, context) = extract_query(initial_query);
    let mut completion_tokens = 0;
    let chars: Vec<char> = context.chars().collect();
    for chunk in chars.chunks(100000) {
        let chunk: String = chunk.iter().collect();
        let (info, tokens) = extract_key_information(system_prompt, &chunk, &query, ctx).await?;
        completion_tokens += tokens;
        for i in info {
            memory.add(i);
        }
    }
    let relevant = memory.get_relevant(&query, 10).map_err(|e| anyhow!(e))?;
    #[rustfmt::skip]
    let prompt = format!(
        concat!(
            "\n",
            "\n",
            "I asked my assistant to read and analyse the above content page by page to help you complete this task. These are margin notes left on each page:\n",
            "'''text\n",
            "{p0}\n",
            "'''\n",
            "Read again the note(s), take a deep breath and answer the query.\n",
            "{p1}\n",
        ),
        p0 = py::json::repr_str_list(&relevant), p1 = query,
    );
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", system_prompt), msg("user", &prompt)]
        }))
        .await?;
    let final_response = response
        .content_at(0)?
        .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
    completion_tokens += response.completion_tokens()?;
    Ok((
        json!(py::strings::strip(&final_response)),
        completion_tokens,
    ))
}

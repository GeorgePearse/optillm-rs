//! scikit-learn's `TfidfVectorizer()` defaults and `cosine_similarity`, plus
//! numpy's `argsort` tie order, as used by the memory and spl plugins.

use std::collections::BTreeMap;

use regex::Regex;

/// `TfidfVectorizer()` defaults: lowercase, token_pattern `(?u)\b\w\w+\b`,
/// smooth_idf, l2 norm, raw term counts.
pub struct Tfidf {
    vocab: BTreeMap<String, usize>,
    idf: Vec<f64>,
    stop_words: bool,
}

fn tokenize(doc: &str, stop_words: bool) -> Vec<String> {
    let re = Regex::new(r"\b\w\w+\b").unwrap();
    let lower = doc.to_lowercase();
    re.find_iter(&lower)
        .map(|m| m.as_str().to_string())
        .filter(|t| !stop_words || !super::stop_words::ENGLISH_STOP_WORDS.contains(&t.as_str()))
        .collect()
}

/// Sparse row as (feature index, value) sorted by index.
pub type Row = Vec<(usize, f64)>;

impl Tfidf {
    /// `TfidfVectorizer().fit_transform(docs)`. Fails like sklearn when the vocabulary is empty.
    pub fn fit_transform(docs: &[String]) -> Result<(Self, Vec<Row>), String> {
        Self::fit_transform_opts(docs, false)
    }

    /// `TfidfVectorizer(stop_words='english' if stop_words else None).fit_transform(docs)`.
    pub fn fit_transform_opts(
        docs: &[String],
        stop_words: bool,
    ) -> Result<(Self, Vec<Row>), String> {
        let tokenized: Vec<Vec<String>> = docs.iter().map(|d| tokenize(d, stop_words)).collect();
        let mut terms: Vec<&String> = tokenized.iter().flatten().collect();
        terms.sort();
        terms.dedup();
        if terms.is_empty() {
            return Err("empty vocabulary; perhaps the documents only contain stop words".into());
        }
        let vocab: BTreeMap<String, usize> = terms
            .iter()
            .enumerate()
            .map(|(i, t)| ((*t).clone(), i))
            .collect();
        let n = docs.len() as f64;
        let mut df = vec![0usize; vocab.len()];
        for toks in &tokenized {
            let mut seen: Vec<usize> = toks.iter().map(|t| vocab[t]).collect();
            seen.sort();
            seen.dedup();
            for i in seen {
                df[i] += 1;
            }
        }
        let idf: Vec<f64> = df
            .iter()
            .map(|d| ((1.0 + n) / (1.0 + *d as f64)).ln() + 1.0)
            .collect();
        let me = Tfidf {
            vocab,
            idf,
            stop_words,
        };
        let rows = tokenized.iter().map(|t| me.row(t)).collect();
        Ok((me, rows))
    }

    fn row(&self, tokens: &[String]) -> Row {
        let mut counts: BTreeMap<usize, f64> = BTreeMap::new();
        for t in tokens {
            if let Some(&i) = self.vocab.get(t) {
                *counts.entry(i).or_insert(0.0) += 1.0;
            }
        }
        let mut row: Row = counts
            .into_iter()
            .map(|(i, c)| (i, c * self.idf[i]))
            .collect();
        let norm = row.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
        if norm > 0.0 {
            for (_, v) in row.iter_mut() {
                *v /= norm;
            }
        }
        row
    }

    /// `transform([doc])[0]`.
    pub fn transform(&self, doc: &str) -> Row {
        self.row(&tokenize(doc, self.stop_words))
    }
}

/// `cosine_similarity(a, b)` for single sparse rows.
pub fn cosine(a: &Row, b: &Row) -> f64 {
    let na = a.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    let nb = b.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    let (mut i, mut j, mut dot) = (0, 0, 0.0);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Equal => {
                dot += (a[i].1 / na) * (b[j].1 / nb);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    dot
}

/// `np.argsort(values)` ascending. numpy's default introsort uses insertion
/// sort below 16 elements, which is stable; larger inputs are sorted stably
/// here too (numpy's tie order there is unspecified).
pub fn argsort(values: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..values.len()).collect();
    idx.sort_by(|a, b| {
        values[*a]
            .partial_cmp(&values[*b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    idx
}

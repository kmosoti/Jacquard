//! Pure-Rust Okapi BM25 inverted search index over [`Article`] catalogs.

use std::collections::{BTreeMap, HashMap};

use crate::model::Article;

const BM25_K1: f64 = 1.5;
const BM25_B: f64 = 0.75;

const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "is", "it", "of", "on",
    "or", "that", "the", "to", "with",
];

/// Ranked search result returned by [`SearchIndex::search`].
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Canonical article route (for example `/wiki/architecture`).
    pub route: String,
    /// Article title.
    pub title: String,
    /// Concise summary or extracted snippet.
    pub summary: String,
    /// Okapi BM25 relevance score (higher is more relevant).
    pub score: f64,
}

struct IndexedDoc {
    route: String,
    title: String,
    summary: String,
    doc_len: f64,
}

/// Pure-Rust BM25 inverted index built over a slice of [`Article`]s.
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    docs: Vec<IndexedDoc>,
    postings: BTreeMap<String, Vec<(usize, f64)>>,
    avg_doc_len: f64,
}

impl std::fmt::Debug for IndexedDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedDoc")
            .field("route", &self.route)
            .field("doc_len", &self.doc_len)
            .finish()
    }
}

impl Clone for IndexedDoc {
    fn clone(&self) -> Self {
        Self {
            route: self.route.clone(),
            title: self.title.clone(),
            summary: self.summary.clone(),
            doc_len: self.doc_len,
        }
    }
}

impl SearchIndex {
    /// Builds a deterministic BM25 inverted index from `articles` (skipping drafts).
    #[must_use]
    pub fn build(articles: &[Article]) -> Self {
        let mut docs = Vec::new();
        let mut postings: BTreeMap<String, Vec<(usize, f64)>> = BTreeMap::new();
        let mut total_len = 0.0f64;

        for article in articles.iter().filter(|a| !a.meta.draft) {
            let doc_id = docs.len();
            let mut tf: HashMap<String, f64> = HashMap::new();
            let mut token_count = 0usize;

            for tok in tokenize(&article.meta.title) {
                *tf.entry(tok).or_insert(0.0) += 3.0;
                token_count += 3;
            }
            for tag in &article.meta.tags {
                for tok in tokenize(tag) {
                    *tf.entry(tok).or_insert(0.0) += 2.0;
                    token_count += 2;
                }
            }
            for tok in tokenize(&article.meta.summary) {
                *tf.entry(tok).or_insert(0.0) += 1.5;
                token_count += 1;
            }
            for tok in tokenize(&article.raw_markdown) {
                *tf.entry(tok).or_insert(0.0) += 1.0;
                token_count += 1;
            }

            let doc_len = (token_count.max(1)) as f64;
            total_len += doc_len;
            docs.push(IndexedDoc {
                route: article.route.clone(),
                title: article.meta.title.clone(),
                summary: article.meta.summary.clone(),
                doc_len,
            });
            for (term, freq) in tf {
                postings.entry(term).or_default().push((doc_id, freq));
            }
        }

        let avg_doc_len = if docs.is_empty() {
            1.0
        } else {
            total_len / docs.len() as f64
        };
        Self {
            docs,
            postings,
            avg_doc_len,
        }
    }

    /// Number of indexed non-draft documents.
    #[must_use]
    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    /// Number of distinct terms in the inverted index.
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.postings.len()
    }

    /// Queries the BM25 index and returns up to `limit` hits ranked by descending score.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        if self.docs.is_empty() || limit == 0 {
            return Vec::new();
        }
        let terms = tokenize(query);
        if terms.is_empty() {
            return Vec::new();
        }
        let n = self.docs.len() as f64;
        let mut scores: HashMap<usize, f64> = HashMap::new();

        for term in terms {
            let Some(list) = self.postings.get(&term) else {
                continue;
            };
            let df = list.len() as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for &(doc_id, tf) in list {
                let dl = self.docs[doc_id].doc_len;
                let denom = tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (dl / self.avg_doc_len));
                let delta = idf * (tf * (BM25_K1 + 1.0)) / denom;
                *scores.entry(doc_id).or_insert(0.0) += delta;
            }
        }

        let mut ranked: Vec<(usize, f64)> = scores.into_iter().collect();
        ranked.sort_by(|&(ia, sa), &(ib, sb)| {
            sb.total_cmp(&sa)
                .then_with(|| self.docs[ia].route.cmp(&self.docs[ib].route))
        });
        ranked.truncate(limit);
        ranked
            .into_iter()
            .map(|(idx, score)| {
                let doc = &self.docs[idx];
                SearchHit {
                    route: doc.route.clone(),
                    title: doc.title.clone(),
                    summary: doc.summary.clone(),
                    score,
                }
            })
            .collect()
    }
}

/// Tokenizes `input` into lowercase ASCII alphanumeric terms, dropping stop words.
#[must_use]
pub fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            if cur.len() > 1 && !STOP_WORDS.contains(&cur.as_str()) {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() > 1 && !STOP_WORDS.contains(&cur.as_str()) {
        out.push(cur);
    }
    out
}

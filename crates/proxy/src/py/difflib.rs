//! Port of `difflib.SequenceMatcher` (including the default `autojunk`
//! heuristic) so similarity-based clustering picks the same clusters as
//! upstream.

use std::collections::HashMap;
use std::hash::Hash;

pub struct SequenceMatcher<T: Eq + Hash + Clone> {
    a: Vec<T>,
    b: Vec<T>,
    b2j: HashMap<T, Vec<usize>>,
    matching_blocks: Option<Vec<(usize, usize, usize)>>,
}

impl SequenceMatcher<char> {
    /// `SequenceMatcher(None, a, b)` over two strings.
    pub fn from_strs(a: &str, b: &str) -> Self {
        Self::new(a.chars().collect(), b.chars().collect())
    }
}

impl<T: Eq + Hash + Clone> SequenceMatcher<T> {
    pub fn new(a: Vec<T>, b: Vec<T>) -> Self {
        let mut m = SequenceMatcher {
            a,
            b,
            b2j: HashMap::new(),
            matching_blocks: None,
        };
        m.chain_b();
        m
    }

    fn chain_b(&mut self) {
        let mut b2j: HashMap<T, Vec<usize>> = HashMap::new();
        for (i, elt) in self.b.iter().enumerate() {
            b2j.entry(elt.clone()).or_default().push(i);
        }
        // autojunk: purge popular elements when len(b) >= 200.
        let n = self.b.len();
        if n >= 200 {
            let ntest = n / 100 + 1;
            let popular: Vec<T> = b2j
                .iter()
                .filter(|(_, idxs)| idxs.len() > ntest)
                .map(|(k, _)| k.clone())
                .collect();
            for elt in popular {
                b2j.remove(&elt);
            }
        }
        self.b2j = b2j;
    }

    fn find_longest_match(
        &self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> (usize, usize, usize) {
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0usize);
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        for i in alo..ahi {
            let mut newj2len: HashMap<usize, usize> = HashMap::new();
            if let Some(idxs) = self.b2j.get(&self.a[i]) {
                for &j in idxs {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = if j > 0 {
                        j2len.get(&(j - 1)).copied().unwrap_or(0) + 1
                    } else {
                        1
                    };
                    newj2len.insert(j, k);
                    if k > bestsize {
                        besti = i + 1 - k;
                        bestj = j + 1 - k;
                        bestsize = k;
                    }
                }
            }
            j2len = newj2len;
        }
        // CPython extends through any element that is not in the explicit junk
        // set; popular elements only stop a match from *starting*.
        while besti > alo && bestj > blo && self.a[besti - 1] == self.b[bestj - 1] {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }
        while besti + bestsize < ahi
            && bestj + bestsize < bhi
            && self.a[besti + bestsize] == self.b[bestj + bestsize]
        {
            bestsize += 1;
        }
        // With isjunk=None the explicit junk set is empty, so CPython's second
        // pair of (junk-extending) loops never fires.
        (besti, bestj, bestsize)
    }

    pub fn get_matching_blocks(&mut self) -> Vec<(usize, usize, usize)> {
        if let Some(mb) = &self.matching_blocks {
            return mb.clone();
        }
        let (la, lb) = (self.a.len(), self.b.len());
        let mut queue = vec![(0usize, la, 0usize, lb)];
        let mut blocks = Vec::new();
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let (i, j, k) = self.find_longest_match(alo, ahi, blo, bhi);
            if k > 0 {
                blocks.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        blocks.sort();
        let mut non_adjacent = Vec::new();
        let (mut i1, mut j1, mut k1) = (0usize, 0usize, 0usize);
        for (i2, j2, k2) in blocks {
            if i1 + k1 == i2 && j1 + k1 == j2 {
                k1 += k2;
            } else {
                if k1 > 0 {
                    non_adjacent.push((i1, j1, k1));
                }
                i1 = i2;
                j1 = j2;
                k1 = k2;
            }
        }
        if k1 > 0 {
            non_adjacent.push((i1, j1, k1));
        }
        non_adjacent.push((la, lb, 0));
        self.matching_blocks = Some(non_adjacent.clone());
        non_adjacent
    }

    /// `SequenceMatcher.ratio()`.
    pub fn ratio(&mut self) -> f64 {
        let matches: usize = self.get_matching_blocks().iter().map(|b| b.2).sum();
        calculate_ratio(matches, self.a.len() + self.b.len())
    }
}

fn calculate_ratio(matches: usize, length: usize) -> f64 {
    if length > 0 {
        2.0 * matches as f64 / length as f64
    } else {
        1.0
    }
}

/// `SequenceMatcher(None, a, b).ratio()` for strings.
pub fn ratio(a: &str, b: &str) -> f64 {
    SequenceMatcher::from_strs(a, b).ratio()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_matches_python() {
        // python: SequenceMatcher(None, "abcd", "bcde").ratio() == 0.75
        assert_eq!(ratio("abcd", "bcde"), 0.75);
        assert_eq!(ratio("", ""), 1.0);
        assert_eq!(
            ratio("The answer is 42.", "The answer is 43."),
            0.9411764705882353
        );
    }
}

//! A bit-exact port of CPython's `random.Random` (MT19937) for the methods the
//! upstream approaches call: `seed(int)`, `random`, `getrandbits`, `choice`,
//! `randint`, `sample` and `shuffle`. With the same seed it yields exactly the
//! same sequence as CPython 3.12, which lets the parity harness pin stochastic
//! approaches (mcts, rstar, mars) to identical call sequences.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

#[derive(Clone)]
pub struct PyRandom {
    mt: [u32; N],
    index: usize,
}

impl std::fmt::Debug for PyRandom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyRandom")
            .field("index", &self.index)
            .finish()
    }
}

impl PyRandom {
    /// `random.Random(seed)` for a non-negative integer seed.
    pub fn new(seed: u64) -> Self {
        let mut r = PyRandom {
            mt: [0; N],
            index: N + 1,
        };
        r.seed(seed);
        r
    }

    /// Seed from OS entropy (used when no deterministic seed is configured).
    pub fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let addr = &nanos as *const u64 as u64;
        Self::new(nanos ^ addr.rotate_left(17))
    }

    /// `random.seed(n)` for an integer `n`.
    pub fn seed(&mut self, seed: u64) {
        let mut key: Vec<u32> = Vec::new();
        let mut n = seed;
        if n == 0 {
            key.push(0);
        }
        while n > 0 {
            key.push((n & 0xffff_ffff) as u32);
            n >>= 32;
        }
        self.init_by_array(&key);
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = N.max(key.len());
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_664_525))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    fn genrand_u32(&mut self) -> u32 {
        if self.index >= N {
            for kk in 0..N {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[(kk + 1) % N] & LOWER_MASK);
                let mut v = self.mt[(kk + M) % N] ^ (y >> 1);
                if y & 1 != 0 {
                    v ^= MATRIX_A;
                }
                self.mt[kk] = v;
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random.random()`.
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand_u32() >> 5) as f64;
        let b = (self.genrand_u32() >> 6) as f64;
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `random.getrandbits(k)` for `k <= 64`.
    pub fn getrandbits(&mut self, k: u32) -> u64 {
        if k == 0 {
            return 0;
        }
        if k <= 32 {
            return (self.genrand_u32() >> (32 - k)) as u64;
        }
        // Little-endian 32-bit words, the last one truncated.
        let mut result: u64 = 0;
        let mut remaining = k;
        let mut shift = 0;
        while remaining > 0 {
            let mut r = self.genrand_u32();
            if remaining < 32 {
                r >>= 32 - remaining;
            }
            result |= (r as u64) << shift;
            shift += 32;
            remaining = remaining.saturating_sub(32);
        }
        result
    }

    /// `Random._randbelow(n)`.
    pub fn randbelow(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let k = usize::BITS - n.leading_zeros();
        let mut r = self.getrandbits(k) as usize;
        while r >= n {
            r = self.getrandbits(k) as usize;
        }
        r
    }

    /// `random.choice(seq)` returning the chosen index.
    pub fn choice_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some(self.randbelow(len))
        }
    }

    /// `random.choice(seq)`.
    pub fn choice<'a, T>(&mut self, seq: &'a [T]) -> Option<&'a T> {
        self.choice_index(seq.len()).map(|i| &seq[i])
    }

    /// `random.randint(a, b)`.
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        a + self.randbelow((b + 1 - a) as usize) as i64
    }

    /// `random.sample(population, k)` returning chosen indices in selection order.
    pub fn sample_indices(&mut self, n: usize, k: usize) -> Vec<usize> {
        assert!(k <= n, "Sample larger than population or is negative");
        let mut result = Vec::with_capacity(k);
        let mut setsize = 21usize;
        if k > 5 {
            let exp = ((k as f64 * 3.0).ln() / 4f64.ln()).ceil() as u32;
            setsize += 4usize.pow(exp);
        }
        if n <= setsize {
            let mut pool: Vec<usize> = (0..n).collect();
            for i in 0..k {
                let j = self.randbelow(n - i);
                result.push(pool[j]);
                pool[j] = pool[n - i - 1];
            }
        } else {
            let mut selected = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n);
                while selected.contains(&j) {
                    j = self.randbelow(n);
                }
                selected.insert(j);
                result.push(j);
            }
        }
        result
    }

    /// `random.sample(population, k)`.
    pub fn sample<T: Clone>(&mut self, population: &[T], k: usize) -> Vec<T> {
        self.sample_indices(population.len(), k)
            .into_iter()
            .map(|i| population[i].clone())
            .collect()
    }

    /// `random.shuffle(x)`.
    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        for i in (1..x.len()).rev() {
            let j = self.randbelow(i + 1);
            x.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values produced by CPython 3.12:
    //   import random; random.seed(42); [random.random() for _ in range(3)]
    //   random.seed(42); [random.randint(1, 10) for _ in range(5)]
    //   random.seed(0); random.sample(range(100), 5); random.seed(7); random.choice('abc') ...
    #[test]
    fn matches_cpython_random() {
        let mut r = PyRandom::new(42);
        let vals: Vec<f64> = (0..3).map(|_| r.random()).collect();
        assert_eq!(
            vals,
            vec![
                0.6394267984578837,
                0.025010755222666936,
                0.27502931836911926
            ]
        );
        let mut r = PyRandom::new(42);
        let ints: Vec<i64> = (0..5).map(|_| r.randint(1, 10)).collect();
        assert_eq!(ints, vec![2, 1, 5, 4, 4]);
        let mut r = PyRandom::new(0);
        assert_eq!(r.sample_indices(100, 5), vec![49, 97, 53, 5, 33]);
        let mut r = PyRandom::new(0);
        assert_eq!(r.sample_indices(10, 3), vec![6, 9, 0]);
    }
}

use rand::{Rng, SeedableRng, rngs::StdRng};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Operation {
    Put { key: String, value: String },
    Get { key: String },
    Cas { key: String, expected: Option<String>, value: String },
}

pub struct Generator {
    pub key_range: usize,
    pub read_ratio: f64,
    pub cas_ratio: f64,
    pub counter: u64,
    pub known_values: HashMap<String, String>,
    rng: StdRng,
}

impl Generator {
    pub fn new(key_range: usize, read_ratio: f64, cas_ratio: f64) -> Self {
        Self {
            key_range,
            read_ratio,
            cas_ratio,
            counter: 0,
            known_values: HashMap::new(),
            rng: StdRng::from_entropy(),
        }
    }

    pub fn next_op(&mut self) -> Operation {
        let rng = &mut self.rng;
        let roll: f64 = rng.gen();
        let key = format!("k{}", rng.gen_range(0..self.key_range));

        let op = if roll < self.read_ratio {
            Operation::Get { key }
        } else if roll < self.read_ratio + self.cas_ratio {
            let expected = self.known_values.get(&key).cloned();
            let value = format!("v{}", self.counter);
            Operation::Cas { key, expected, value }
        } else {
            let value = format!("v{}", self.counter);
            Operation::Put { key, value }
        };

        self.counter += 1;
        op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_op_covers_all_variants_and_keys_in_range() {
        // Use ratios that give every branch a fair share.
        let key_range = 5;
        let mut gen = Generator::new(key_range, 0.4, 0.3);

        let mut saw_put = false;
        let mut saw_get = false;
        let mut saw_cas = false;

        for _ in 0..100 {
            let op = gen.next_op();
            match &op {
                Operation::Put { key, value } => {
                    saw_put = true;
                    let idx: usize = key.strip_prefix('k').unwrap().parse().unwrap();
                    assert!(idx < key_range, "Put key out of range: {key}");
                    assert!(value.starts_with('v'), "Put value has wrong prefix: {value}");
                }
                Operation::Get { key } => {
                    saw_get = true;
                    let idx: usize = key.strip_prefix('k').unwrap().parse().unwrap();
                    assert!(idx < key_range, "Get key out of range: {key}");
                }
                Operation::Cas { key, value, .. } => {
                    saw_cas = true;
                    let idx: usize = key.strip_prefix('k').unwrap().parse().unwrap();
                    assert!(idx < key_range, "Cas key out of range: {key}");
                    assert!(value.starts_with('v'), "Cas value has wrong prefix: {value}");
                }
            }
        }

        assert!(saw_put, "no Put produced in 100 ops");
        assert!(saw_get, "no Get produced in 100 ops");
        assert!(saw_cas, "no Cas produced in 100 ops");
    }

    #[test]
    fn counter_increments_each_call() {
        let mut gen = Generator::new(3, 0.0, 0.0); // all Puts
        for i in 0..10 {
            let op = gen.next_op();
            if let Operation::Put { value, .. } = op {
                assert_eq!(value, format!("v{i}"));
            }
        }
        assert_eq!(gen.counter, 10);
    }
}

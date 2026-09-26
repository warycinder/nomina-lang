use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};

use crate::parser::{Expr, Grammar};

/// How many nested rule references `eval` will follow before giving up.
/// Real grammars bottom out in a handful of hops; this only exists to
/// turn an accidental rule cycle into a clean error instead of a stack
/// overflow, since the validator does not (yet) reject cycles.
pub const MAX_DEPTH: usize = 200;

/// A small, fast, non-cryptographic PRNG (xorshift64*). Good enough for
/// picking alternatives; it does not need to resist prediction, just
/// avoid visible bias or short cycles.
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seeds from the standard library's hashmap randomness, which on
    /// every supported platform is itself seeded from OS entropy. That
    /// gives a different sequence per run without pulling in a crate
    /// just to read `/dev/urandom` or call `getrandom`.
    pub fn new() -> Self {
        let a = RandomState::new().build_hasher().finish();
        let b = RandomState::new().build_hasher().finish();
        let seed = a ^ b.rotate_left(32);
        Rng { state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `0..bound`, without the modulo bias a plain `% bound`
    /// would introduce.
    fn below(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return 0;
        }
        let bound = bound as u64;
        let limit = u64::MAX - (u64::MAX % bound);
        loop {
            let x = self.next_u64();
            if x < limit {
                return (x % bound) as usize;
            }
        }
    }

    /// True with probability `1/n`.
    fn one_in(&mut self, n: u64) -> bool {
        self.below(n as usize) == 0
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new()
    }
}

/// Returned when evaluating `name` follows more rule references than
/// `MAX_DEPTH` allows, which in practice means two or more rules refer to
/// each other with no literal to bottom out on.
#[derive(Debug)]
pub struct RecursionLimitExceeded;

/// Generates one random name by evaluating the grammar's `name` rule.
/// Assumes `parser::validate` has already run, so every reference is
/// known to resolve to a rule that exists.
pub fn generate(grammar: &Grammar, rng: &mut Rng) -> Result<String, RecursionLimitExceeded> {
    let rules: HashMap<&str, &Expr> = grammar
        .rules
        .iter()
        .map(|r| (r.name.as_str(), &r.expr))
        .collect();
    let start = rules.get("name").expect("validated grammar has a `name` rule");
    let mut out = String::new();
    eval(start, &rules, rng, 0, &mut out)?;
    Ok(out)
}

fn eval(
    expr: &Expr,
    rules: &HashMap<&str, &Expr>,
    rng: &mut Rng,
    depth: usize,
    out: &mut String,
) -> Result<(), RecursionLimitExceeded> {
    match expr {
        Expr::Literal(s, _) => {
            out.push_str(s);
            Ok(())
        }
        Expr::Ref(name, _) => {
            if depth >= MAX_DEPTH {
                return Err(RecursionLimitExceeded);
            }
            let target = rules
                .get(name.as_str())
                .expect("validated grammar has no dangling references");
            eval(target, rules, rng, depth + 1, out)
        }
        Expr::Seq(items) => {
            for item in items {
                eval(item, rules, rng, depth, out)?;
            }
            Ok(())
        }
        Expr::Alt(branches) => {
            let pick = rng.below(branches.len());
            eval(&branches[pick], rules, rng, depth, out)
        }
        Expr::Opt(inner) => {
            if rng.one_in(2) {
                Ok(())
            } else {
                eval(inner, rules, rng, depth, out)
            }
        }
    }
}

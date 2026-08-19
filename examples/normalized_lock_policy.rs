use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

trait LockOrderingPolicy {
    fn encoding(&self) -> &'static str;
    fn allows(&self, held: &str, acquired: &str) -> Option<bool>;
}

#[allow(dead_code, unused_imports)]
mod successor_dag {
    include!("lock_order_policy.rs");

    pub mod normalized {
        use std::collections::BTreeMap;
        use std::error::Error;

        use super::{RankRule, parse_rank_rules};

        pub struct Policy {
            rules: BTreeMap<String, RankRule>,
        }

        impl Policy {
            pub fn parse(source: &str) -> Result<Self, Box<dyn Error>> {
                Ok(Self {
                    rules: parse_rank_rules(source)?,
                })
            }

            pub fn allows(&self, held: &str, acquired: &str) -> Option<bool> {
                let held = self.rules.get(held)?;
                self.rules.get(acquired)?;
                Some(held.followers.contains(acquired))
            }
        }
    }
}

#[allow(dead_code, unused_imports)]
mod numeric_rank {
    include!("numeric_lock_order_policy.rs");

    pub mod normalized {
        use std::collections::BTreeMap;
        use std::error::Error;

        use super::parse_exact_name_ranks;

        pub struct Policy {
            ranks: BTreeMap<String, u64>,
        }

        impl Policy {
            pub fn parse(source: &str) -> Result<Self, Box<dyn Error>> {
                Ok(Self {
                    ranks: parse_exact_name_ranks(source)?,
                })
            }

            pub fn allows(&self, held: &str, acquired: &str) -> Option<bool> {
                let held = self.ranks.get(held)?;
                let acquired = self.ranks.get(acquired)?;
                Some(acquired >= held)
            }
        }
    }
}

impl LockOrderingPolicy for successor_dag::normalized::Policy {
    fn encoding(&self) -> &'static str {
        "successor-dag"
    }

    fn allows(&self, held: &str, acquired: &str) -> Option<bool> {
        successor_dag::normalized::Policy::allows(self, held, acquired)
    }
}

impl LockOrderingPolicy for numeric_rank::normalized::Policy {
    fn encoding(&self) -> &'static str {
        "numeric-nondecreasing"
    }

    fn allows(&self, held: &str, acquired: &str) -> Option<bool> {
        numeric_rank::normalized::Policy::allows(self, held, acquired)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("normalized-lock-policy: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or(usage())?;
    let policy_path = PathBuf::from(args.next().ok_or(usage())?);
    let held = args.next().ok_or(usage())?;
    let acquired = args.next().ok_or(usage())?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let source = fs::read_to_string(&policy_path)?;
    match mode.as_str() {
        "dag" => report(
            &successor_dag::normalized::Policy::parse(&source)?,
            &held,
            &acquired,
        ),
        "numeric" => report(
            &numeric_rank::normalized::Policy::parse(&source)?,
            &held,
            &acquired,
        ),
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn report(policy: &impl LockOrderingPolicy, held: &str, acquired: &str) {
    println!("NORMALIZED LOCK ORDER POLICY");
    println!("  encoding: {}", policy.encoding());
    println!("  held: {held}");
    println!("  acquired: {acquired}");
    match policy.allows(held, acquired) {
        Some(true) => println!("  decision: ALLOW"),
        Some(false) => println!("  decision: DENY"),
        None => println!("  decision: UNKNOWN"),
    }
}

fn usage() -> &'static str {
    "usage: normalized_lock_policy MODE POLICY_FILE HELD ACQUIRED\n\
MODE: dag | numeric"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successor_dag_uses_same_allows_contract() {
        let source = r#"
            macro_rules! define_lock_ranks { ($($tt:tt)*) => {} }
            define_lock_ranks! {
                rank FIRST "Owner::first" followed by { SECOND }
                rank SECOND "Owner::second" followed by { }
            }
        "#;
        let policy = successor_dag::normalized::Policy::parse(source).unwrap();
        assert_eq!(policy.allows("FIRST", "SECOND"), Some(true));
        assert_eq!(policy.allows("SECOND", "FIRST"), Some(false));
        assert_eq!(policy.allows("MISSING", "SECOND"), None);
    }

    #[test]
    fn numeric_ranks_use_same_allows_contract() {
        let source = r#"
        {
          "rank_map": [
            {"numeric_rank": 10, "from_name_samples": ["first"]},
            {"numeric_rank": 40, "from_name_samples": ["second"]}
          ]
        }
        "#;
        let policy = numeric_rank::normalized::Policy::parse(source).unwrap();
        assert_eq!(policy.allows("first", "second"), Some(true));
        assert_eq!(policy.allows("second", "first"), Some(false));
        assert_eq!(policy.allows("missing", "second"), None);
    }
}

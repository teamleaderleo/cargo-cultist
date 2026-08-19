use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use syn::{Expr, ExprCall, ImplItem, Item, Pat, Stmt};

#[derive(Debug, Clone, Eq, PartialEq)]
struct LockBinding {
    variable: String,
    lock_name: String,
    rank: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Acquisition {
    guard: String,
    lock_variable: String,
    lock_name: String,
    rank: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("numeric-lock-order-policy: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: numeric_lock_order_policy REPO INVENTORY_JSON SOURCE_RS FUNCTION")?,
    )
    .canonicalize()?;
    let inventory = PathBuf::from(
        args.next()
            .ok_or("usage: numeric_lock_order_policy REPO INVENTORY_JSON SOURCE_RS FUNCTION")?,
    );
    let source = PathBuf::from(
        args.next()
            .ok_or("usage: numeric_lock_order_policy REPO INVENTORY_JSON SOURCE_RS FUNCTION")?,
    );
    let function_name = args
        .next()
        .ok_or("usage: numeric_lock_order_policy REPO INVENTORY_JSON SOURCE_RS FUNCTION")?;
    if args.next().is_some() {
        return Err(
            "usage: numeric_lock_order_policy REPO INVENTORY_JSON SOURCE_RS FUNCTION".into(),
        );
    }

    let policy_source = fs::read_to_string(root.join(&inventory))?;
    let exact_name_ranks = parse_exact_name_ranks(&policy_source)?;
    let source_text = fs::read_to_string(root.join(&source))?;
    let function = find_function(&source_text, &function_name)?;
    let bindings = collect_lock_bindings(&function, &exact_name_ranks);
    let acquisitions = collect_acquisitions(&function, &bindings);

    println!("NUMERIC LOCK ORDER POLICY PROBE");
    println!("  repository: {}", root.display());
    println!("  policy inventory: {}", inventory.display());
    println!("  source: {}", source.display());
    println!("  function: {function_name}");
    println!("  exact ranked lock-name samples: {}", exact_name_ranks.len());

    if bindings.is_empty() {
        println!("\nOBSERVATION");
        println!(
            "  No supported literal lock definitions matched exact ranked names from the inventory."
        );
        print_boundary();
        return Ok(());
    }

    println!("\nLOCK DEFINITIONS");
    for binding in bindings.values() {
        println!(
            "  `{}` -> `{}` rank {}",
            binding.variable, binding.lock_name, binding.rank
        );
    }

    if acquisitions.is_empty() {
        println!("\nOBSERVATION");
        println!("  No supported named guard acquisitions were found.");
        print_boundary();
        return Ok(());
    }

    println!("\nACQUISITIONS");
    let mut highest_held: Option<&Acquisition> = None;
    let mut violations = Vec::new();
    for acquisition in &acquisitions {
        println!(
            "  guard `{}` acquires `{}` rank {}",
            acquisition.guard, acquisition.lock_name, acquisition.rank
        );
        if let Some(highest) = highest_held
            && acquisition.rank < highest.rank
        {
            violations.push((highest.clone(), acquisition.clone()));
        }
        if highest_held.is_none_or(|highest| acquisition.rank > highest.rank) {
            highest_held = Some(acquisition);
        }
    }

    if violations.is_empty() {
        println!("\nOBSERVATION");
        println!(
            "  Supported acquisitions are nondecreasing by numeric rank, matching the ordered-rank policy."
        );
    } else {
        println!("\nFINDING: numeric lock-rank order contradicted by lexical acquisition");
        for (held, acquired) in violations {
            println!("\nPROVEN / DERIVED");
            println!(
                "  `{}` (rank {}) is held when `{}` (rank {}) is acquired.",
                held.lock_name, held.rank, acquired.lock_name, acquired.rank
            );
            println!(
                "  The acquired rank {} is lower than the highest supported rank {} already held.",
                acquired.rank, held.rank
            );
            println!("\nQUESTION");
            println!(
                "  Is this descending rank acquisition intentional, or should this function follow the repository's ordered lock hierarchy?"
            );
        }
    }

    print_boundary();
    Ok(())
}

fn parse_exact_name_ranks(source: &str) -> Result<BTreeMap<String, u64>, Box<dyn Error>> {
    let value: Value = serde_json::from_str(source)?;
    let rank_map = value
        .get("rank_map")
        .and_then(Value::as_array)
        .ok_or("inventory has no `rank_map` array")?;

    let mut names = BTreeMap::new();
    for entry in rank_map {
        let rank = entry
            .get("rank")
            .and_then(Value::as_u64)
            .ok_or("rank_map entry has no numeric `rank`")?;
        let Some(samples) = entry.get("from_name_samples").and_then(Value::as_object) else {
            continue;
        };
        for (name, sample_rank) in samples {
            let Some(sample_rank) = sample_rank.as_u64() else {
                continue;
            };
            if sample_rank == rank {
                names.insert(name.clone(), rank);
            }
        }
    }

    if names.is_empty() {
        return Err("inventory contains no exact `from_name_samples` rank mappings".into());
    }
    Ok(names)
}

fn find_function(source: &str, function_name: &str) -> Result<syn::ItemFn, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    find_function_in_items(&file.items, function_name)
        .ok_or_else(|| format!("function `{function_name}` not found").into())
}

fn find_function_in_items(items: &[Item], function_name: &str) -> Option<syn::ItemFn> {
    for item in items {
        match item {
            Item::Fn(function) if function.sig.ident == function_name => return Some(function.clone()),
            Item::Mod(module) => {
                if let Some((_, nested)) = &module.content
                    && let Some(function) = find_function_in_items(nested, function_name)
                {
                    return Some(function);
                }
            }
            Item::Impl(item_impl) => {
                for item in &item_impl.items {
                    if let ImplItem::Fn(function) = item
                        && function.sig.ident == function_name
                    {
                        return Some(syn::ItemFn {
                            attrs: function.attrs.clone(),
                            vis: function.vis.clone(),
                            sig: function.sig.clone(),
                            block: Box::new(function.block.clone()),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_lock_bindings(
    function: &syn::ItemFn,
    exact_name_ranks: &BTreeMap<String, u64>,
) -> BTreeMap<String, LockBinding> {
    let mut bindings = BTreeMap::new();
    for statement in &function.block.stmts {
        let Stmt::Local(local) = statement else {
            continue;
        };
        let Pat::Ident(pattern) = &local.pat else {
            continue;
        };
        let Some(init) = &local.init else {
            continue;
        };
        let Some(lock_name) = literal_lock_name(&init.expr) else {
            continue;
        };
        let Some(rank) = exact_name_ranks.get(&lock_name).copied() else {
            continue;
        };
        bindings.insert(
            pattern.ident.to_string(),
            LockBinding {
                variable: pattern.ident.to_string(),
                lock_name,
                rank,
            },
        );
    }
    bindings
}

fn literal_lock_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Call(call) => {
            if constructor_last_segment(call)
                .is_some_and(|name| matches!(name.as_str(), "new" | "with_name"))
                && let Some(name) = call.args.first().and_then(literal_string)
            {
                return Some(name);
            }
            for argument in &call.args {
                if let Some(name) = literal_lock_name(argument) {
                    return Some(name);
                }
            }
            None
        }
        Expr::Paren(paren) => literal_lock_name(&paren.expr),
        Expr::Group(group) => literal_lock_name(&group.expr),
        Expr::Reference(reference) => literal_lock_name(&reference.expr),
        _ => None,
    }
}

fn constructor_last_segment(call: &ExprCall) -> Option<String> {
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn literal_string(expr: &Expr) -> Option<String> {
    let Expr::Lit(literal) = expr else {
        return None;
    };
    let syn::Lit::Str(value) = &literal.lit else {
        return None;
    };
    Some(value.value())
}

fn collect_acquisitions(
    function: &syn::ItemFn,
    bindings: &BTreeMap<String, LockBinding>,
) -> Vec<Acquisition> {
    let mut acquisitions = Vec::new();
    for statement in &function.block.stmts {
        let Stmt::Local(local) = statement else {
            continue;
        };
        let Pat::Ident(pattern) = &local.pat else {
            continue;
        };
        let Some(init) = &local.init else {
            continue;
        };
        let Some(lock_variable) = acquired_lock_variable(&init.expr) else {
            continue;
        };
        let Some(binding) = bindings.get(&lock_variable) else {
            continue;
        };
        acquisitions.push(Acquisition {
            guard: pattern.ident.to_string(),
            lock_variable,
            lock_name: binding.lock_name.clone(),
            rank: binding.rank,
        });
    }
    acquisitions
}

fn acquired_lock_variable(expr: &Expr) -> Option<String> {
    match expr {
        Expr::MethodCall(call) if call.method == "lock" => match &*call.receiver {
            Expr::Path(path) if path.path.segments.len() == 1 => {
                Some(path.path.segments[0].ident.to_string())
            }
            _ => None,
        },
        Expr::MethodCall(call) => acquired_lock_variable(&call.receiver),
        Expr::Paren(paren) => acquired_lock_variable(&paren.expr),
        Expr::Group(group) => acquired_lock_variable(&group.expr),
        Expr::Reference(reference) => acquired_lock_variable(&reference.expr),
        _ => None,
    }
}

fn print_boundary() {
    println!("\nEVIDENCE BOUNDARY");
    println!("  Numeric ranks come only from exact lock-name samples in the repository inventory.");
    println!("  This adapter treats lower-to-higher numeric rank as the ordered hierarchy; the target repository's runtime checker independently enforces `new_rank < highest_held` as a violation.");
    println!("  The lexical extractor handles literal lock constructors and named `.lock().unwrap()` guard acquisitions in one function.");
    println!("  Prefix inference, async lock futures, explicit drops, nested control flow, aliases, and dynamic lock names remain outside this first adapter.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVENTORY: &str = r#"
    {
      "rank_map": [
        {"rank": 10, "from_name_samples": {"config_cache": 10}},
        {"rank": 30, "from_name_samples": {"regions_table": 30}},
        {"rank": 40, "from_name_samples": {"tasks_queue": 40}}
      ]
    }
    "#;

    #[test]
    fn parses_exact_inventory_samples() {
        let ranks = parse_exact_name_ranks(INVENTORY).unwrap();
        assert_eq!(ranks["config_cache"], 10);
        assert_eq!(ranks["regions_table"], 30);
        assert_eq!(ranks["tasks_queue"], 40);
    }

    #[test]
    fn extracts_nested_literal_lock_constructors() {
        let function = find_function(
            r#"
            fn example() {
                let tasks_lock = Arc::new(ContendedMutex::new("tasks_queue", 0));
                let _tasks_guard = tasks_lock.lock().unwrap();
            }
            "#,
            "example",
        )
        .unwrap();
        let ranks = parse_exact_name_ranks(INVENTORY).unwrap();
        let bindings = collect_lock_bindings(&function, &ranks);
        assert_eq!(bindings["tasks_lock"].rank, 40);
        let acquisitions = collect_acquisitions(&function, &bindings);
        assert_eq!(acquisitions.len(), 1);
        assert_eq!(acquisitions[0].lock_name, "tasks_queue");
    }

    #[test]
    fn descending_sequence_is_detectable() {
        let acquisitions = [
            Acquisition {
                guard: "tasks_guard".to_string(),
                lock_variable: "tasks_lock".to_string(),
                lock_name: "tasks_queue".to_string(),
                rank: 40,
            },
            Acquisition {
                guard: "config_guard".to_string(),
                lock_variable: "config_lock".to_string(),
                lock_name: "config_cache".to_string(),
                rank: 10,
            },
        ];
        let mut highest = 0;
        let mut violations = 0;
        for acquisition in acquisitions {
            if acquisition.rank < highest {
                violations += 1;
            }
            highest = highest.max(acquisition.rank);
        }
        assert_eq!(violations, 1);
    }
}

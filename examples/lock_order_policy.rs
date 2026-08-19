use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use proc_macro2::{Literal, TokenTree};
use syn::{Expr, ExprCall, ExprMethodCall, ImplItem, Item, LitStr, Member, Pat, Stmt};

#[derive(Debug, Clone, Eq, PartialEq)]
struct RankRule {
    name: String,
    member: String,
    followers: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Acquisition {
    guard: String,
    field: String,
    rank: String,
    line: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Violation {
    held: Acquisition,
    acquired: Acquisition,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("lock-order-policy: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_policy REPO RANK_RS SOURCE_RS FUNCTION")?,
    )
    .canonicalize()?;
    let rank_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_policy REPO RANK_RS SOURCE_RS FUNCTION")?,
    );
    let source_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_policy REPO RANK_RS SOURCE_RS FUNCTION")?,
    );
    let function_name = args
        .next()
        .ok_or("usage: lock_order_policy REPO RANK_RS SOURCE_RS FUNCTION")?;
    if args.next().is_some() {
        return Err("usage: lock_order_policy REPO RANK_RS SOURCE_RS FUNCTION".into());
    }
    if rank_path.is_absolute() || source_path.is_absolute() {
        return Err("RANK_RS and SOURCE_RS must be repository-relative".into());
    }

    let rank_source = fs::read_to_string(root.join(&rank_path))?;
    let source = fs::read_to_string(root.join(&source_path))?;
    let ranks = parse_rank_rules(&rank_source)?;
    let field_to_rank = unique_field_ranks(&ranks);
    let function = find_impl_function(&source, &function_name)?;
    let (acquisitions, violations, releases) = analyze_function(function, &ranks, &field_to_rank);

    println!("LOCK ORDER POLICY PROBE");
    println!("  repository: {}", root.display());
    println!("  rank policy: {}", rank_path.display());
    println!("  source: {}", source_path.display());
    println!("  function: {function_name}");
    println!("  declared rank rules: {}", ranks.len());

    if acquisitions.is_empty() {
        println!("\nOBSERVATION");
        println!("  No supported named lock-guard acquisitions were found.");
        print_boundary();
        return Ok(());
    }

    println!("\nACQUISITIONS");
    for acquisition in &acquisitions {
        println!(
            "  line {:>5}: guard `{}` acquires `{}` as rank `{}`",
            acquisition.line, acquisition.guard, acquisition.field, acquisition.rank
        );
    }

    if !releases.is_empty() {
        println!("\nEXPLICIT RELEASES");
        for (line, guard) in releases {
            println!("  line {line:>5}: drop({guard})");
        }
    }

    if violations.is_empty() {
        println!("\nOBSERVATION");
        println!(
            "  Every supported acquisition made while another named guard remained held is permitted by the declared rank DAG."
        );
    } else {
        println!("\nFINDING: declared lock-rank order contradicted by lexical acquisition");
        for violation in &violations {
            println!("\nPROVEN / DERIVED");
            println!(
                "  `{}` ({}) is still held when `{}` ({}) is acquired.",
                violation.held.field,
                violation.held.rank,
                violation.acquired.field,
                violation.acquired.rank
            );
            println!(
                "  The rank DAG does not list `{}` as an allowed follower of `{}`.",
                violation.acquired.rank, violation.held.rank
            );
            println!(
                "  acquisition lines: {} -> {}",
                violation.held.line, violation.acquired.line
            );

            println!("\nQUESTION");
            println!(
                "  Is this inverse lock acquisition intentional, or should this function follow the repository's declared rank order?"
            );
        }
    }

    print_boundary();
    Ok(())
}

fn parse_rank_rules(source: &str) -> Result<BTreeMap<String, RankRule>, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    let mut rules = BTreeMap::new();

    for item in file.items {
        let Item::Macro(item_macro) = item else {
            continue;
        };
        if !item_macro.mac.path.is_ident("define_lock_ranks") {
            continue;
        }

        let tokens: Vec<_> = item_macro.mac.tokens.into_iter().collect();
        let mut index = 0;
        while index < tokens.len() {
            if is_punct(&tokens[index], '#') {
                index += 1;
                if index < tokens.len() && matches!(tokens[index], TokenTree::Group(_)) {
                    index += 1;
                }
                continue;
            }
            if !is_ident(&tokens[index], "rank") {
                index += 1;
                continue;
            }

            let name = token_ident(tokens.get(index + 1)).ok_or("rank name missing")?;
            let member = token_string_literal(tokens.get(index + 2)).ok_or("rank member missing")?;
            if !tokens.get(index + 3).is_some_and(|token| is_ident(token, "followed"))
                || !tokens.get(index + 4).is_some_and(|token| is_ident(token, "by"))
            {
                return Err(format!("rank `{name}` is missing `followed by`").into());
            }
            let Some(TokenTree::Group(group)) = tokens.get(index + 5) else {
                return Err(format!("rank `{name}` follower group missing").into());
            };
            let followers = group
                .stream()
                .into_iter()
                .filter_map(|token| match token {
                    TokenTree::Ident(ident) => Some(ident.to_string()),
                    _ => None,
                })
                .collect();

            rules.insert(
                name.clone(),
                RankRule {
                    name,
                    member,
                    followers,
                },
            );
            index += 6;
        }
    }

    if rules.is_empty() {
        return Err("no `define_lock_ranks!` rules found".into());
    }
    Ok(rules)
}

fn unique_field_ranks(ranks: &BTreeMap<String, RankRule>) -> BTreeMap<String, String> {
    let mut candidates = BTreeMap::<String, Vec<String>>::new();
    for rule in ranks.values() {
        let Some(field) = rule.member.rsplit("::").next() else {
            continue;
        };
        candidates
            .entry(field.to_string())
            .or_default()
            .push(rule.name.clone());
    }

    candidates
        .into_iter()
        .filter_map(|(field, ranks)| {
            (ranks.len() == 1).then(|| (field, ranks.into_iter().next().unwrap()))
        })
        .collect()
}

fn find_impl_function(source: &str, function_name: &str) -> Result<syn::ImplItemFn, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    for item in file.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        for item in item_impl.items {
            let ImplItem::Fn(function) = item else {
                continue;
            };
            if function.sig.ident == function_name {
                return Ok(function);
            }
        }
    }
    Err(format!("function `{function_name}` not found in impl blocks").into())
}

fn analyze_function(
    function: syn::ImplItemFn,
    ranks: &BTreeMap<String, RankRule>,
    field_to_rank: &BTreeMap<String, String>,
) -> (Vec<Acquisition>, Vec<Violation>, Vec<(usize, String)>) {
    let mut held = BTreeMap::<String, Acquisition>::new();
    let mut acquisitions = Vec::new();
    let mut violations = Vec::new();
    let mut releases = Vec::new();

    for statement in function.block.stmts {
        if let Some(acquisition) = local_acquisition(&statement, field_to_rank) {
            for prior in held.values() {
                let allowed = ranks
                    .get(&prior.rank)
                    .is_some_and(|rule| rule.followers.contains(&acquisition.rank));
                if !allowed {
                    violations.push(Violation {
                        held: prior.clone(),
                        acquired: acquisition.clone(),
                    });
                }
            }
            held.insert(acquisition.guard.clone(), acquisition.clone());
            acquisitions.push(acquisition);
            continue;
        }

        if let Some((line, guard)) = explicit_drop(&statement) {
            held.remove(&guard);
            releases.push((line, guard));
        }
    }

    (acquisitions, violations, releases)
}

fn local_acquisition(
    statement: &Stmt,
    field_to_rank: &BTreeMap<String, String>,
) -> Option<Acquisition> {
    let Stmt::Local(local) = statement else {
        return None;
    };
    let Pat::Ident(pattern) = &local.pat else {
        return None;
    };
    let init = local.init.as_ref()?;
    let Expr::MethodCall(call) = peel_expr(&init.expr) else {
        return None;
    };
    if !matches!(call.method.to_string().as_str(), "lock" | "read" | "write") {
        return None;
    }
    let field = receiver_field(&call.receiver)?;
    let rank = field_to_rank.get(&field)?.clone();
    Some(Acquisition {
        guard: pattern.ident.to_string(),
        field,
        rank,
        line: call.method.span().start().line,
    })
}

fn explicit_drop(statement: &Stmt) -> Option<(usize, String)> {
    let Stmt::Expr(expr, _) = statement else {
        return None;
    };
    let Expr::Call(call) = peel_expr(expr) else {
        return None;
    };
    let Expr::Path(path) = peel_expr(&call.func) else {
        return None;
    };
    if !path.path.is_ident("drop") || call.args.len() != 1 {
        return None;
    }
    let Expr::Path(argument) = peel_expr(call.args.first()?) else {
        return None;
    };
    if argument.path.segments.len() != 1 {
        return None;
    }
    Some((
        call.func.span().start().line,
        argument.path.segments[0].ident.to_string(),
    ))
}

fn receiver_field(expr: &Expr) -> Option<String> {
    match peel_expr(expr) {
        Expr::Field(field) => match &field.member {
            Member::Named(ident) => Some(ident.to_string()),
            Member::Unnamed(_) => None,
        },
        _ => None,
    }
}

fn peel_expr(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(reference) => peel_expr(&reference.expr),
        Expr::Paren(paren) => peel_expr(&paren.expr),
        Expr::Group(group) => peel_expr(&group.expr),
        _ => expr,
    }
}

fn token_ident(token: Option<&TokenTree>) -> Option<String> {
    match token? {
        TokenTree::Ident(ident) => Some(ident.to_string()),
        _ => None,
    }
}

fn token_string_literal(token: Option<&TokenTree>) -> Option<String> {
    let TokenTree::Literal(literal) = token? else {
        return None;
    };
    parse_lit_str(literal)
}

fn parse_lit_str(literal: &Literal) -> Option<String> {
    syn::parse_str::<LitStr>(&literal.to_string())
        .ok()
        .map(|value| value.value())
}

fn is_ident(token: &TokenTree, expected: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident == expected)
}

fn is_punct(token: &TokenTree, expected: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == expected)
}

fn print_boundary() {
    println!("\nEVIDENCE BOUNDARY");
    println!("  Rank policy is read from the repository's `define_lock_ranks!` invocation.");
    println!("  Field-to-rank matching requires a unique rank member field name.");
    println!("  Acquisition tracking covers simple named guards initialized by `.lock()`, `.read()`, or `.write()` at the function's top statement level.");
    println!("  Explicit `drop(guard)` ends that held interval. Temporary guards, helper-returned guards, nested blocks, aliases, and control-flow joins remain outside this first probe.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANKS: &str = r#"
        macro_rules! define_lock_ranks { ($($tt:tt)*) => {} }
        define_lock_ranks! {
            rank FIRST "Owner::first" followed by { SECOND }
            rank SECOND "Owner::second" followed by { }
        }
    "#;

    #[test]
    fn parses_rank_dag() {
        let ranks = parse_rank_rules(RANKS).unwrap();
        assert!(ranks["FIRST"].followers.contains("SECOND"));
        assert!(ranks["SECOND"].followers.is_empty());
        assert_eq!(ranks["FIRST"].member, "Owner::first");
    }

    #[test]
    fn valid_order_has_no_violation() {
        let ranks = parse_rank_rules(RANKS).unwrap();
        let fields = unique_field_ranks(&ranks);
        let function = find_impl_function(
            r#"
            impl Thing {
                fn work(&self) {
                    let first_guard = self.first.lock();
                    let second_guard = self.second.write();
                    drop(second_guard);
                    drop(first_guard);
                }
            }
            "#,
            "work",
        )
        .unwrap();
        let (_, violations, _) = analyze_function(function, &ranks, &fields);
        assert!(violations.is_empty());
    }

    #[test]
    fn inverse_order_is_reported() {
        let ranks = parse_rank_rules(RANKS).unwrap();
        let fields = unique_field_ranks(&ranks);
        let function = find_impl_function(
            r#"
            impl Thing {
                fn work(&self) {
                    let second_guard = self.second.lock();
                    let first_guard = self.first.write();
                    drop(first_guard);
                    drop(second_guard);
                }
            }
            "#,
            "work",
        )
        .unwrap();
        let (_, violations, _) = analyze_function(function, &ranks, &fields);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].held.rank, "SECOND");
        assert_eq!(violations[0].acquired.rank, "FIRST");
    }

    #[test]
    fn explicit_drop_removes_held_guard() {
        let ranks = parse_rank_rules(RANKS).unwrap();
        let fields = unique_field_ranks(&ranks);
        let function = find_impl_function(
            r#"
            impl Thing {
                fn work(&self) {
                    let second_guard = self.second.lock();
                    drop(second_guard);
                    let first_guard = self.first.write();
                    drop(first_guard);
                }
            }
            "#,
            "work",
        )
        .unwrap();
        let (_, violations, releases) = analyze_function(function, &ranks, &fields);
        assert!(violations.is_empty());
        assert_eq!(releases.len(), 2);
    }
}

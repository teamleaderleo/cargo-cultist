use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprStruct, ImplItem, Item, Member, Pat, Stmt};

#[derive(Debug, Clone, Eq, PartialEq)]
struct RankRule {
    name: String,
    member: String,
    followers: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HeldEffect {
    owner: String,
    field: String,
    rank: String,
    line: usize,
    origin: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Violation {
    held: HeldEffect,
    acquired: HeldEffect,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("lock-helper-effects: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    )
    .canonicalize()?;
    let rank_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    );
    let source_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    );
    let helper_name = args
        .next()
        .ok_or("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?;
    let caller_name = args
        .next()
        .ok_or("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?;
    if args.next().is_some() {
        return Err("usage: lock_helper_effects REPO RANK_RS SOURCE_RS HELPER CALLER".into());
    }
    if rank_path.is_absolute() || source_path.is_absolute() {
        return Err("RANK_RS and SOURCE_RS must be repository-relative".into());
    }

    let rank_source = fs::read_to_string(root.join(&rank_path))?;
    let source = fs::read_to_string(root.join(&source_path))?;
    let ranks = parse_rank_rules(&rank_source)?;
    let field_to_rank = unique_field_ranks(&ranks);
    let helper = find_impl_function(&source, &helper_name)?;
    let caller = find_impl_function(&source, &caller_name)?;
    let helper_effects = summarize_returned_guards(&helper, &field_to_rank);
    let (events, violations) = analyze_caller(
        caller,
        &helper_name,
        &helper_effects,
        &ranks,
        &field_to_rank,
    );

    println!("LOCK HELPER EFFECT PROBE");
    println!("  repository: {}", root.display());
    println!("  rank policy: {}", rank_path.display());
    println!("  source: {}", source_path.display());
    println!("  helper: {helper_name}");
    println!("  caller: {caller_name}");

    println!("\nHELPER SUMMARY");
    if helper_effects.is_empty() {
        println!("  No supported locally acquired guard was stored into a returned struct value.");
    } else {
        for effect in &helper_effects {
            println!(
                "  `{helper_name}` returns while `{}` / `{}` remains owned by the return value (rank `{}`).",
                effect.owner, effect.field, effect.rank
            );
        }
    }

    println!("\nCALLER EVENTS");
    for event in &events {
        println!(
            "  line {:>5}: {} holds/acquires `{}` as `{}` ({})",
            event.line, event.origin, event.field, event.rank, event.owner
        );
    }

    if violations.is_empty() {
        println!("\nOBSERVATION");
        println!(
            "  Every supported direct or helper-carried acquisition follows the declared successor rule for the most recently acquired rank still held."
        );
    } else {
        println!("\nFINDING: helper-carried lock effect contradicts declared rank order");
        for violation in &violations {
            println!("\nDERIVED");
            println!(
                "  `{}` ({}) remains held via {} when `{}` ({}) is acquired.",
                violation.held.field,
                violation.held.rank,
                violation.held.origin,
                violation.acquired.field,
                violation.acquired.rank
            );
            println!(
                "  The rank DAG does not list `{}` as a follower of `{}`.",
                violation.acquired.rank, violation.held.rank
            );
            println!("\nQUESTION");
            println!("  Is this helper-carried inverse acquisition intentional?");
        }
    }

    println!("\nBOUNDARY");
    println!(
        "  This probe recognizes top-level named acquisitions, helper calls bound to locals, and helpers that store a locally acquired guard into a struct expression."
    );
    println!(
        "  Aliases, guards moved through additional helpers, conditional lifetimes, nested control flow, and arbitrary ownership transfers remain outside this slice."
    );
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
            if !tokens
                .get(index + 3)
                .is_some_and(|token| is_ident(token, "followed"))
                || !tokens
                    .get(index + 4)
                    .is_some_and(|token| is_ident(token, "by"))
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
        if let Some(field) = rule.member.rsplit("::").next() {
            candidates
                .entry(field.to_string())
                .or_default()
                .push(rule.name.clone());
        }
    }
    candidates
        .into_iter()
        .filter_map(|(field, names)| (names.len() == 1).then(|| (field, names[0].clone())))
        .collect()
}

fn find_impl_function(source: &str, name: &str) -> Result<syn::ImplItemFn, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    for item in file.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        for item in item_impl.items {
            let ImplItem::Fn(function) = item else {
                continue;
            };
            if function.sig.ident == name {
                return Ok(function);
            }
        }
    }
    Err(format!("function `{name}` not found in impl blocks").into())
}

fn summarize_returned_guards(
    helper: &syn::ImplItemFn,
    field_to_rank: &BTreeMap<String, String>,
) -> Vec<HeldEffect> {
    let acquisitions: BTreeMap<_, _> = helper
        .block
        .stmts
        .iter()
        .filter_map(|stmt| local_acquisition(stmt, field_to_rank))
        .map(|effect| (effect.owner.clone(), effect))
        .collect();

    let mut visitor = StructGuardVisitor {
        guard_names: acquisitions.keys().cloned().collect(),
        retained: BTreeSet::new(),
    };
    visitor.visit_block(&helper.block);

    acquisitions
        .into_iter()
        .filter_map(|(guard, mut effect)| {
            visitor.retained.contains(&guard).then(|| {
                effect.origin = format!("helper return `{}`", helper.sig.ident);
                effect
            })
        })
        .collect()
}

struct StructGuardVisitor {
    guard_names: BTreeSet<String>,
    retained: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for StructGuardVisitor {
    fn visit_expr_struct(&mut self, node: &'ast ExprStruct) {
        for field in &node.fields {
            if let Expr::Path(path) = peel_expr(&field.expr)
                && path.path.segments.len() == 1
            {
                let name = path.path.segments[0].ident.to_string();
                if self.guard_names.contains(&name) {
                    self.retained.insert(name);
                }
            }
        }
        visit::visit_expr_struct(self, node);
    }
}

fn analyze_caller(
    caller: syn::ImplItemFn,
    helper_name: &str,
    helper_effects: &[HeldEffect],
    ranks: &BTreeMap<String, RankRule>,
    field_to_rank: &BTreeMap<String, String>,
) -> (Vec<HeldEffect>, Vec<Violation>) {
    let mut held = Vec::<HeldEffect>::new();
    let mut helper_owners = BTreeMap::<String, usize>::new();
    let mut events = Vec::new();
    let mut violations = Vec::new();

    for statement in caller.block.stmts {
        if let Some((binding, line)) = local_helper_call(&statement, helper_name) {
            for effect in helper_effects {
                let mut effect = effect.clone();
                effect.owner = binding.clone();
                effect.line = line;
                effect.origin = format!("helper result `{binding}`");
                check_and_push(&mut held, &mut violations, &effect, ranks);
                events.push(effect);
            }
            helper_owners.insert(binding, helper_effects.len());
            continue;
        }

        if let Some(effect) = local_acquisition(&statement, field_to_rank) {
            check_and_push(&mut held, &mut violations, &effect, ranks);
            events.push(effect);
            continue;
        }

        if let Some(binding) = explicit_drop(&statement) {
            if let Some(count) = helper_owners.remove(&binding) {
                let mut remaining = count;
                held.retain(|effect| {
                    if effect.owner == binding && remaining > 0 {
                        remaining -= 1;
                        false
                    } else {
                        true
                    }
                });
            } else {
                held.retain(|effect| effect.owner != binding);
            }
        }
    }

    (events, violations)
}

fn check_and_push(
    held: &mut Vec<HeldEffect>,
    violations: &mut Vec<Violation>,
    acquired: &HeldEffect,
    ranks: &BTreeMap<String, RankRule>,
) {
    if let Some(prior) = held.last() {
        let allowed = ranks
            .get(&prior.rank)
            .is_some_and(|rule| rule.followers.contains(&acquired.rank));
        if !allowed {
            violations.push(Violation {
                held: prior.clone(),
                acquired: acquired.clone(),
            });
        }
    }
    held.push(acquired.clone());
}

fn local_acquisition(
    statement: &Stmt,
    field_to_rank: &BTreeMap<String, String>,
) -> Option<HeldEffect> {
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
    Some(HeldEffect {
        owner: pattern.ident.to_string(),
        field,
        rank,
        line: call.method.span().start().line,
        origin: "direct acquisition".to_string(),
    })
}

fn local_helper_call(statement: &Stmt, helper_name: &str) -> Option<(String, usize)> {
    let Stmt::Local(local) = statement else {
        return None;
    };
    let Pat::Ident(pattern) = &local.pat else {
        return None;
    };
    let init = local.init.as_ref()?;
    let mut visitor = MethodFinder {
        name: helper_name,
        line: None,
    };
    visitor.visit_expr(&init.expr);
    visitor.line.map(|line| (pattern.ident.to_string(), line))
}

struct MethodFinder<'a> {
    name: &'a str,
    line: Option<usize>,
}

impl<'ast> Visit<'ast> for MethodFinder<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.line.is_none() && node.method == self.name {
            self.line = Some(node.method.span().start().line);
        }
        visit::visit_expr_method_call(self, node);
    }
}

fn explicit_drop(statement: &Stmt) -> Option<String> {
    let Stmt::Expr(expr, _) = statement else {
        return None;
    };
    let Expr::Call(ExprCall { func, args, .. }) = peel_expr(expr) else {
        return None;
    };
    let Expr::Path(path) = peel_expr(func) else {
        return None;
    };
    if !path.path.is_ident("drop") || args.len() != 1 {
        return None;
    }
    let Expr::Path(argument) = peel_expr(args.first()?) else {
        return None;
    };
    (argument.path.segments.len() == 1).then(|| argument.path.segments[0].ident.to_string())
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
        Expr::Try(value) => peel_expr(&value.expr),
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
    syn::parse_str::<syn::LitStr>(&literal.to_string())
        .ok()
        .map(|literal| literal.value())
}

fn is_ident(token: &TokenTree, expected: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident == expected)
}

fn is_punct(token: &TokenTree, expected: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> BTreeMap<String, RankRule> {
        BTreeMap::from([
            (
                "A".to_string(),
                RankRule {
                    name: "A".to_string(),
                    member: "Owner::a".to_string(),
                    followers: BTreeSet::from(["B".to_string()]),
                },
            ),
            (
                "B".to_string(),
                RankRule {
                    name: "B".to_string(),
                    member: "Owner::b".to_string(),
                    followers: BTreeSet::new(),
                },
            ),
        ])
    }

    fn functions(source: &str) -> (syn::ImplItemFn, syn::ImplItemFn) {
        let file = syn::parse_file(source).unwrap();
        let Item::Impl(item_impl) = file.items.into_iter().next().unwrap() else {
            panic!("expected impl");
        };
        let mut functions = item_impl.items.into_iter().filter_map(|item| match item {
            ImplItem::Fn(function) => Some(function),
            _ => None,
        });
        (functions.next().unwrap(), functions.next().unwrap())
    }

    #[test]
    fn carries_guard_stored_in_helper_return() {
        let source = r#"
            impl Owner {
                fn helper(&self) -> Token<'_> {
                    let a_guard = self.a.lock();
                    Token { a_guard }
                }

                fn caller(&self) {
                    let token = self.helper();
                    let b_guard = self.b.lock();
                    drop(b_guard);
                    drop(token);
                }
            }
        "#;
        let (helper, caller) = functions(source);
        let ranks = rules();
        let fields = unique_field_ranks(&ranks);
        let effects = summarize_returned_guards(&helper, &fields);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].rank, "A");
        let (_, violations) = analyze_caller(caller, "helper", &effects, &ranks, &fields);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_inverse_helper_carried_edge() {
        let source = r#"
            impl Owner {
                fn helper(&self) -> Token<'_> {
                    let b_guard = self.b.lock();
                    Token { b_guard }
                }

                fn caller(&self) {
                    let token = self.helper();
                    let a_guard = self.a.lock();
                    drop(a_guard);
                    drop(token);
                }
            }
        "#;
        let (helper, caller) = functions(source);
        let ranks = rules();
        let fields = unique_field_ranks(&ranks);
        let effects = summarize_returned_guards(&helper, &fields);
        let (_, violations) = analyze_caller(caller, "helper", &effects, &ranks, &fields);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].held.rank, "B");
        assert_eq!(violations[0].acquired.rank, "A");
    }
}

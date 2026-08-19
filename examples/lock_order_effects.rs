#[allow(dead_code, unused_imports)]
mod policy {
    include!("lock_order_policy.rs");

    pub mod effects {
        use std::collections::{BTreeMap, BTreeSet};
        use std::error::Error;

        use syn::visit::{self, Visit};
        use syn::{Expr, ExprStruct, Pat, Stmt};

        use super::{
            Acquisition, RankRule, explicit_drop, find_impl_function, local_acquisition,
            parse_rank_rules, peel_expr, unique_field_ranks,
        };

        #[derive(Debug, Clone, Eq, PartialEq)]
        struct Event {
            acquisition: Acquisition,
            origin: String,
        }

        #[derive(Debug, Clone, Eq, PartialEq)]
        struct EffectViolation {
            held: Event,
            acquired: Event,
        }

        #[derive(Debug, Default)]
        struct EffectReport {
            helper_effects: Vec<Acquisition>,
            events: Vec<Event>,
            violations: Vec<EffectViolation>,
        }

        pub fn analyze(
            rank_source: &str,
            source: &str,
            helper_name: &str,
            caller_name: &str,
        ) -> Result<(), Box<dyn Error>> {
            let ranks = parse_rank_rules(rank_source)?;
            let field_to_rank = unique_field_ranks(&ranks);
            let helper = find_impl_function(source, helper_name)?;
            let caller = find_impl_function(source, caller_name)?;
            let report = analyze_functions(helper, caller, helper_name, &ranks, &field_to_rank);

            println!("LOCK ORDER EFFECTS PROBE");
            println!("  helper: {helper_name}");
            println!("  caller: {caller_name}");

            println!("\nHELPER EFFECTS");
            if report.helper_effects.is_empty() {
                println!(
                    "  No supported locally acquired guard is retained by the helper's returned struct value."
                );
            }
            for effect in &report.helper_effects {
                println!(
                    "  `{helper_name}` returns with `{}` held as rank `{}`.",
                    effect.field, effect.rank
                );
            }

            println!("\nCALLER EVENTS");
            for event in &report.events {
                println!(
                    "  line {:>5}: {} -> `{}` as `{}`",
                    event.acquisition.line,
                    event.origin,
                    event.acquisition.field,
                    event.acquisition.rank
                );
            }

            if report.violations.is_empty() {
                println!("\nOBSERVATION");
                println!(
                    "  Every supported direct or helper-carried acquisition follows the declared successor rule."
                );
            } else {
                println!("\nFINDING: helper-carried lock effect contradicts declared rank order");
                for violation in &report.violations {
                    println!("\nDERIVED");
                    println!(
                        "  `{}` ({}) remains held via {} when `{}` ({}) is acquired via {}.",
                        violation.held.acquisition.field,
                        violation.held.acquisition.rank,
                        violation.held.origin,
                        violation.acquired.acquisition.field,
                        violation.acquired.acquisition.rank,
                        violation.acquired.origin
                    );
                    println!("\nQUESTION");
                    println!("  Is this helper-carried inverse acquisition intentional?");
                }
            }

            println!("\nBOUNDARY");
            println!(
                "  Helper effects cover top-level named acquisitions retained in a struct expression returned from the helper."
            );
            println!(
                "  Caller tracking covers top-level direct acquisitions, helper calls bound to locals, and explicit drop(binding)."
            );
            println!(
                "  Aliases, nested control flow, additional ownership transfers, and general lifetime analysis remain outside this slice."
            );
            Ok(())
        }

        fn analyze_functions(
            helper: syn::ImplItemFn,
            caller: syn::ImplItemFn,
            helper_name: &str,
            ranks: &BTreeMap<String, RankRule>,
            field_to_rank: &BTreeMap<String, String>,
        ) -> EffectReport {
            let helper_effects = retained_helper_acquisitions(&helper, field_to_rank);
            let mut report = EffectReport {
                helper_effects: helper_effects.clone(),
                ..EffectReport::default()
            };
            let mut held = Vec::<Event>::new();
            let mut helper_bindings = BTreeSet::<String>::new();

            for statement in caller.block.stmts {
                if let Some((binding, line)) = helper_binding(&statement, helper_name) {
                    helper_bindings.insert(binding.clone());
                    for effect in &helper_effects {
                        let mut acquisition = effect.clone();
                        acquisition.guard = binding.clone();
                        acquisition.line = line;
                        let event = Event {
                            acquisition,
                            origin: format!("helper result `{binding}`"),
                        };
                        check_event(&mut held, &mut report.violations, &event, ranks);
                        report.events.push(event);
                    }
                    continue;
                }

                if let Some(acquisition) = local_acquisition(&statement, field_to_rank) {
                    let event = Event {
                        acquisition,
                        origin: "direct acquisition".to_string(),
                    };
                    check_event(&mut held, &mut report.violations, &event, ranks);
                    report.events.push(event);
                    continue;
                }

                if let Some((_line, binding)) = explicit_drop(&statement) {
                    held.retain(|event| event.acquisition.guard != binding);
                    helper_bindings.remove(&binding);
                }
            }

            report
        }

        fn retained_helper_acquisitions(
            helper: &syn::ImplItemFn,
            field_to_rank: &BTreeMap<String, String>,
        ) -> Vec<Acquisition> {
            let acquisitions: BTreeMap<_, _> = helper
                .block
                .stmts
                .iter()
                .filter_map(|statement| local_acquisition(statement, field_to_rank))
                .map(|acquisition| (acquisition.guard.clone(), acquisition))
                .collect();
            let mut visitor = RetainedGuardVisitor {
                candidates: acquisitions.keys().cloned().collect(),
                retained: BTreeSet::new(),
            };
            visitor.visit_block(&helper.block);
            acquisitions
                .into_iter()
                .filter_map(|(guard, acquisition)| {
                    visitor.retained.contains(&guard).then_some(acquisition)
                })
                .collect()
        }

        struct RetainedGuardVisitor {
            candidates: BTreeSet<String>,
            retained: BTreeSet<String>,
        }

        impl<'ast> Visit<'ast> for RetainedGuardVisitor {
            fn visit_expr_struct(&mut self, node: &'ast ExprStruct) {
                for field in &node.fields {
                    if let Expr::Path(path) = peel_expr(&field.expr)
                        && path.path.segments.len() == 1
                    {
                        let name = path.path.segments[0].ident.to_string();
                        if self.candidates.contains(&name) {
                            self.retained.insert(name);
                        }
                    }
                }
                visit::visit_expr_struct(self, node);
            }
        }

        fn helper_binding(statement: &Stmt, helper_name: &str) -> Option<(String, usize)> {
            let Stmt::Local(local) = statement else {
                return None;
            };
            let Pat::Ident(pattern) = &local.pat else {
                return None;
            };
            let init = local.init.as_ref()?;
            let mut visitor = HelperCallVisitor {
                helper_name,
                line: None,
            };
            visitor.visit_expr(&init.expr);
            visitor.line.map(|line| (pattern.ident.to_string(), line))
        }

        struct HelperCallVisitor<'a> {
            helper_name: &'a str,
            line: Option<usize>,
        }

        impl<'ast> Visit<'ast> for HelperCallVisitor<'_> {
            fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
                if self.line.is_none() && node.method == self.helper_name {
                    self.line = Some(node.method.span().start().line);
                }
                visit::visit_expr_method_call(self, node);
            }
        }

        fn check_event(
            held: &mut Vec<Event>,
            violations: &mut Vec<EffectViolation>,
            acquired: &Event,
            ranks: &BTreeMap<String, RankRule>,
        ) {
            if let Some(prior) = held.last() {
                let allowed = ranks
                    .get(&prior.acquisition.rank)
                    .is_some_and(|rule| rule.followers.contains(&acquired.acquisition.rank));
                if !allowed {
                    violations.push(EffectViolation {
                        held: prior.clone(),
                        acquired: acquired.clone(),
                    });
                }
            }
            held.push(acquired.clone());
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

            fn report(source: &str) -> EffectReport {
                let ranks = parse_rank_rules(RANKS).unwrap();
                let fields = unique_field_ranks(&ranks);
                let helper = find_impl_function(source, "helper").unwrap();
                let caller = find_impl_function(source, "caller").unwrap();
                analyze_functions(helper, caller, "helper", &ranks, &fields)
            }

            #[test]
            fn carries_allowed_helper_effect() {
                let result = report(
                    r#"
                    impl Owner {
                        fn helper(&self) -> Token<'_> {
                            let first_guard = self.first.lock();
                            Token { first_guard }
                        }

                        fn caller(&self) {
                            let token = self.helper();
                            let second_guard = self.second.lock();
                            drop(second_guard);
                            drop(token);
                        }
                    }
                    "#,
                );
                assert_eq!(result.helper_effects.len(), 1);
                assert!(result.violations.is_empty());
            }

            #[test]
            fn reports_inverse_helper_effect() {
                let result = report(
                    r#"
                    impl Owner {
                        fn helper(&self) -> Token<'_> {
                            let second_guard = self.second.lock();
                            Token { second_guard }
                        }

                        fn caller(&self) {
                            let token = self.helper();
                            let first_guard = self.first.lock();
                            drop(first_guard);
                            drop(token);
                        }
                    }
                    "#,
                );
                assert_eq!(result.violations.len(), 1);
                assert_eq!(result.violations[0].held.acquisition.rank, "SECOND");
                assert_eq!(result.violations[0].acquired.acquisition.rank, "FIRST");
            }
        }
    }
}

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("lock-order-effects: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    )
    .canonicalize()?;
    let rank_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    );
    let source_path = PathBuf::from(
        args.next()
            .ok_or("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?,
    );
    let helper_name = args
        .next()
        .ok_or("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?;
    let caller_name = args
        .next()
        .ok_or("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER")?;
    if args.next().is_some() {
        return Err("usage: lock_order_effects REPO RANK_RS SOURCE_RS HELPER CALLER".into());
    }
    if rank_path.is_absolute() || source_path.is_absolute() {
        return Err("RANK_RS and SOURCE_RS must be repository-relative".into());
    }

    let rank_source = fs::read_to_string(root.join(&rank_path))?;
    let source = fs::read_to_string(root.join(&source_path))?;
    policy::effects::analyze(&rank_source, &source, &helper_name, &caller_name)
}

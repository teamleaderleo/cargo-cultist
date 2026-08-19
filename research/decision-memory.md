# Decision memory: reviewed repository lessons as future context

Date: 2026-08-19

Status: research proof for #75 / #10 / #74. Storage format and final product location remain provisional.

## Question

Can one reviewed repository decision become deterministic version-controlled memory that a later agent recovers for the exact code it applies to?

```text
finding / repository question
-> reviewed human decision
-> rationale + authority stored in Git
-> later agent targets related code
-> resolver returns applicable decision evidence
```

The record is evidence. Resolving it does not suppress a finding, prove that the rationale remains universally correct, or promote a project-wide rule.

## Research carrier

This branch uses one JSON file per decision under:

```text
research/decision-memory/
```

That location and JSON schema are research choices. They can be replaced after the lifecycle is better understood.

The first self-dogfood record captures the already-reviewed Cargo Cultist decision that raw historical co-change remains association evidence until stronger evidence earns promotion:

```text
id:    history-cochange-remains-association-v1
scope: src/history.rs
refs:  #34, #39, #19
```

## Resolver

```text
cargo run --example decision_memory -- RECORDS_DIR TARGET
```

The resolver:

- resolves the target inside its Git repository;
- requires the supplied records directory to live inside that repository;
- reads sorted `.json` records;
- requires records to be regular files rather than symlinks;
- validates schema and required fields;
- requires globally unique record IDs in the supplied memory set;
- validates scope as canonical repository-relative `/`-separated path syntax;
- matches scope by path components;
- returns source file, full rationale, and plural authority references;
- fails closed on malformed or unsupported memory.

## Scope semantics

The v1 research scope is deliberately small:

```text
scope.path_prefix = "src/history.rs"
```

Canonical syntax rejects ambiguity such as:

```text
../src/history.rs
src/./history.rs
src//history.rs
src\history.rs
/absolute/path
```

A directory scope such as `src/history` can match descendants. A file scope does not string-prefix-match a neighbor such as `src/history.rs.bak`.

There is no glob, item, symbol, package, rename, expiry, or finding-family syntax yet.

## Record identity

`id` is stable identity inside the supplied memory set. Two records with the same ID are an error even when their scopes differ.

That is intentional: future Git history, review links, migration, or promotion need one unambiguous referent.

## File provenance

A `.json` symlink is rejected. Research decision memory is expected to be content reviewed in the repository itself, so an in-tree path that dereferences to mutable/out-of-tree content is outside this proof.

This says nothing yet about whether the containing commit was merged, reviewed, signed, or otherwise accepted. Accepted-versus-proposed authority remains a separate problem.

## Authority model

The record carries plural evidence references, while the Git event that accepts the record is conceptually separate.

An agent may propose:

```text
scope + reason + authority references
```

and ordinary repository review can accept or reject that proposal. A later product should expose acceptance provenance explicitly rather than trusting the record's self-declared references as sufficient authority.

## First discriminator

Positive:

```text
TARGET=src/history.rs
-> exactly history-cochange-remains-association-v1
-> authority refs #34, #39, #19
```

Unrelated:

```text
TARGET=src/main.rs
-> zero decisions
```

Fail-closed controls:

```text
schema 99                   -> error
scope ../src/history.rs     -> error
scope src/./history.rs      -> error
duplicate record ID         -> error
symlinked .json record      -> error
```

## Agent lifecycle

The larger loop remains:

```text
BEFORE
  brief retrieves applicable reviewed memory

DURING
  diff surfaces live tension with that memory

AFTER
  human-reviewed decision records a newly earned lesson

NEXT TIME
  another agent recovers it without the original chat transcript
```

## Promotion gates

Keep this as research until several harder cases pass:

1. at least two independent decision kinds need the same core fields;
2. scope semantics survive a real rename/refactor;
3. `brief` and `diff` consume decisions without turning them into implicit suppressions;
4. accepted versus merely proposed authority is represented explicitly;
5. one longitudinal replay shows agent B recovering and correctly using a decision left by agent A.

If those experiments prefer a different schema or storage location, replace this carrier rather than preserving research compatibility.

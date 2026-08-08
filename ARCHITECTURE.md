# Architecture

```text
gql-source -> gql-syntax (lexer + Rowan CST) -> gql-ast (typed view lowering)
                                             -> gql-sema -> gql-ir
                                             \-> gql-compiler

gql-core    pure public language surface
gql-catalog ISO catalog declarations only
gql-reasoning backend/provider contracts
gql-ascent  optional derived-relation provider -> gql-reasoning
gql         ergonomic facade, default features = []
```

## Frontend authority

gql-syntax is the sole concrete-syntax authority. The lexer emits tokens and
the parser emits Rowan events directly into one lossless Rowan tree. Every
source byte, including whitespace, comments, unknown characters, malformed
delimiters, empty input, and trailing newlines, is represented in that tree.
gql-syntax::SyntaxNode and the gql-ast lowering adapter are typed views over
that Rowan tree; they do not construct a second owned CST or reparse source
spans.

Precedence is defined once in the parser event layer. Structural expression
nodes are retained in Rowan and lowered into the graph-semantic IR.

## Graph-semantic boundary

The ISO semantic path is graph-pattern-first. A node-only MATCH is valid and
does not require an edge, relation provider, or backend catalog entry. The
canonical IR has first-class graph, pattern, node, edge, binding, direction,
label, filter, LET, and projection concepts.

gql-catalog contains only ISO catalog, graph, schema, node-type, edge-type,
property, and procedure declarations. Derived relation names, provider
descriptors, derivation witnesses, and Ascent authority live in
gql-reasoning and its optional implementations.

## Purity boundary

`gql-source`, `gql-syntax`, `gql-ast`, `gql-types`, `gql-catalog`, `gql-sema`,
`gql-ir`, `gql-compiler`, and `gql-core` must never depend on Ascent. Cargo
features are additive, so purity-sensitive consumers should depend on
`gql-core` directly.

`gql-ascent` implements the optional reasoning provider contracts. It may affect
which derived tuples are produced outside the ISO frontend. It may not affect
parsing, the graph-semantic IR, or the meaning of valid ISO GQL.

No Ascent DSL is accepted by the GQL parser. Rulesets are registered outside
the language and exposed through typed catalog descriptors.

## Assurance status

The current crates establish dependency direction and the supported ISO
frontend slice. Full ISO/IEC 39075 conformance is not claimed. The feature
ledger records each ISO feature separately from quality capabilities such as
lossless recovery; every entry has a stable identity, normative reference,
positive and negative fixtures, AST/IR status, and validation evidence.

# Architecture

```text
gql-source -> gql-syntax -> gql-ast -> gql-sema -> gql-ir
                                  \-> gql-compiler

gql-core    pure public language surface
gql-ascent  optional derived-relation provider -> gql-catalog/gql-ir
gql         ergonomic facade, default features = []
```

## Purity boundary

`gql-source`, `gql-syntax`, `gql-ast`, `gql-types`, `gql-catalog`, `gql-sema`,
`gql-ir`, `gql-compiler`, and `gql-core` must never depend on Ascent. Cargo
features are additive, so purity-sensitive consumers should depend on
`gql-core` directly.

`gql-ascent` implements backend-neutral catalog contracts. It may affect which
typed relations are available and how derived tuples are produced. It may not
affect parsing or the meaning of valid ISO GQL.

No Ascent DSL is accepted by the GQL parser. Rulesets are registered outside
the language and exposed through typed catalog descriptors.

## Assurance status

The current crates establish dependency direction and foundational types. They
do not claim full ISO/IEC 39075 conformance. Each future feature must be tied to
a standard clause, positive and negative fixtures, AST/IR representation, and
an explicit implementation status.

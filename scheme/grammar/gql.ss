;;; ISO GQL MRR projection bound to the canonical parser-owned grammar.

(import ./core
        ./gql-declaration
        ./parser-authority)
(export mrr-gql-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-gql-grammar iso-gql "ISO GQL"
 mrr-gql-parser-authority)

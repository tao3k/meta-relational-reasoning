;;; openCypher MRR projection bound to the canonical parser-owned grammar.

(import ./core
        ./gql-declaration
        ./parser-authority)
(export mrr-cypher-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-cypher-grammar open-cypher "openCypher"
 mrr-cypher-parser-authority)

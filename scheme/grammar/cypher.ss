;;; openCypher POO grammar instantiated from the single Scheme declaration.

(import ./core
        ./gql-declaration)
(export mrr-cypher-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-cypher-grammar open-cypher "openCypher")

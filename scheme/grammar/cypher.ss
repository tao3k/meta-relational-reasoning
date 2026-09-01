;;; openCypher POO grammar instantiated from the single Scheme declaration.

(import ./core)
(export mrr-cypher-grammar)

(include "gql-declaration.ss")

(with-mrr-gql-declaration
 defmrr-grammar mrr-cypher-grammar open-cypher "openCypher")

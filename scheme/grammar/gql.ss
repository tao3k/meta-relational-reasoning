;;; ISO GQL POO grammar instantiated from the single Scheme declaration.

(import ./core)
(export mrr-gql-grammar)

(include "gql-declaration.ss")

(with-mrr-gql-declaration
 defmrr-grammar mrr-gql-grammar iso-gql "ISO GQL")

;;; ISO GQL POO grammar instantiated from the single Scheme declaration.

(import ./core
        ./gql-declaration)
(export mrr-gql-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-gql-grammar iso-gql "ISO GQL")

;;; Executable contracts for the Gerbil grammar authority.

(import :std/test
        :clan/poo/object
        :poo-flow/src/core/object-syntax
        :meta-relational-reasoning/scheme/grammar/gql
        :meta-relational-reasoning/scheme/grammar/cypher)
(export grammar-authority-test)

(def grammar-authority-test
  (test-suite "MRR Gerbil grammar authority"
    (test-case "ISO GQL is the active POO grammar root"
      (check-equal? (.ref mrr-gql-grammar 'kind) 'mrr-grammar)
      (check-equal? (.ref mrr-gql-grammar 'active?) #t)
      (check-equal? (.ref mrr-gql-grammar 'extends) '())
      (check-equal? (.ref mrr-gql-grammar 'dialect-id) 'iso-gql))
    (test-case "openCypher is an independent active profile"
      (check-equal? (.ref mrr-cypher-grammar 'active?) #t)
      (check-equal? (.ref mrr-cypher-grammar 'extends) '())
      (check-equal? (.ref mrr-cypher-grammar 'dialect-id) 'open-cypher))
    (test-case "both profiles are projections of one declaration"
      (check-equal? (.ref mrr-cypher-grammar 'syntax-kinds)
                    (.ref mrr-gql-grammar 'syntax-kinds))
      (check-equal? (.ref mrr-cypher-grammar 'keywords)
                    (.ref mrr-gql-grammar 'keywords))
      (check-equal? (.ref mrr-cypher-grammar 'prefix-operators)
                    (.ref mrr-gql-grammar 'prefix-operators))
      (check-equal? (.ref mrr-cypher-grammar 'binary-operators)
                    (.ref mrr-gql-grammar 'binary-operators))
      (check-equal? (.ref mrr-cypher-grammar 'parser-entrypoints)
                    (.ref mrr-gql-grammar 'parser-entrypoints))
      (check-equal? (.ref mrr-cypher-grammar 'recoveries)
                    (.ref mrr-gql-grammar 'recoveries)))))

(run-tests! grammar-authority-test)

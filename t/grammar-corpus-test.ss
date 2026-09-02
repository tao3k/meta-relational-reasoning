#!/usr/bin/env gxi
;;; Shared M5 corpus contract for the Gerbil declaration authority.

(import :std/test
        :std/srfi/13
        :clan/poo/object
        :poo-flow/src/core/object-syntax
        :meta-relational-reasoning/scheme/grammar/gql
        :meta-relational-reasoning/scheme/grammar/cypher)
(export grammar-corpus-test)

(def (read-corpus path)
  (call-with-input-file path
    (lambda (port)
      (let loop ((cases '()))
        (let (line (read-line port))
          (if (eof-object? line)
            (reverse cases)
            (let (separator (string-index line #\tab))
              (unless separator
                (error "corpus row must be id<TAB>query" path line))
              (loop
               (cons
                (cons (substring line 0 separator)
                      (substring line (+ separator 1) (string-length line)))
                cases)))))))))

(def (grammar-entrypoint? grammar keyword)
  (let loop ((rows (.ref grammar 'parser-entrypoints)))
    (cond
     ((null? rows) #f)
     ((eq? (caar rows) keyword) #t)
     (else (loop (cdr rows))))))

(def (grammar-recovery? grammar recovery)
  (let loop ((rows (.ref grammar 'recoveries)))
    (cond
     ((null? rows) #f)
     ((eq? (caar rows) recovery) #t)
     (else (loop (cdr rows))))))

(def positive-cases
  (read-corpus "crates/gql-syntax/test-data/parser/m5-positive.tsv"))
(def negative-cases
  (read-corpus "crates/gql-syntax/test-data/parser/m5-negative.tsv"))

(def grammar-corpus-test
  (test-suite "MRR shared Gerbil/Rust grammar corpus"
    (test-case "corpus has exactly 100 positive and 100 negative cases"
      (check-equal? (length positive-cases) 100)
      (check-equal? (length negative-cases) 100))
    (test-case "corpus rows route through declared MATCH RETURN and WHERE owners"
      (check-equal? (.ref mrr-gql-grammar 'active?) #t)
      (check-equal? (grammar-entrypoint? mrr-gql-grammar 'Match) #t)
      (check-equal? (grammar-entrypoint? mrr-gql-grammar 'Return) #t)
      (check-equal? (grammar-entrypoint? mrr-gql-grammar 'Where) #t)
      (check-equal? (grammar-recovery? mrr-gql-grammar 'where-clause) #t)
      (for-each
       (lambda (case)
         (check-equal? (string-prefix? "MATCH " (cdr case)) #t)
         (check-equal? (if (string-contains (cdr case) " RETURN ") #t #f) #t))
       positive-cases)
      (for-each
       (lambda (case)
         (check-equal? (string-prefix? "MATCH " (cdr case)) #t)
         (check-equal? (if (string-contains (cdr case) " WHERE RETURN ") #t #f) #t))
       negative-cases))
    (test-case "openCypher profile shares the exact declaration projection"
      (check-equal?
       (.ref mrr-cypher-grammar 'syntax-kinds)
       (.ref mrr-gql-grammar 'syntax-kinds))
      (check-equal?
       (.ref mrr-cypher-grammar 'parser-entrypoints)
       (.ref mrr-gql-grammar 'parser-entrypoints))
      (check-equal?
       (.ref mrr-cypher-grammar 'recoveries)
       (.ref mrr-gql-grammar 'recoveries)))))

(run-tests! grammar-corpus-test)

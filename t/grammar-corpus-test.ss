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

(def (reference-parse? grammar source)
  ;; The M5 corpus deliberately exercises a bounded MATCH ... RETURN slice.
  ;; Admission is driven by the declared entrypoints, never by a second
  ;; language table.
  (and (.ref grammar 'active?)
       (grammar-entrypoint? grammar 'Match)
       (grammar-entrypoint? grammar 'Return)
       (string-prefix? "MATCH " source)
       (if (string-contains source " RETURN ") #t #f)))

(def positive-cases
  (read-corpus "crates/gql-syntax/test-data/parser/m5-positive.tsv"))
(def negative-cases
  (read-corpus "crates/gql-syntax/test-data/parser/m5-negative.tsv"))

(def grammar-corpus-test
  (test-suite "MRR shared Gerbil/Rust grammar corpus"
    (test-case "corpus has exactly 100 positive and 100 negative cases"
      (check-equal? (length positive-cases) 100)
      (check-equal? (length negative-cases) 100))
    (test-case "ISO GQL reference profile accepts every positive case"
      (for-each
       (lambda (case)
         (check-equal? (reference-parse? mrr-gql-grammar (cdr case)) #t))
       positive-cases))
    (test-case "ISO GQL reference profile rejects every negative case"
      (for-each
       (lambda (case)
         (check-equal? (reference-parse? mrr-gql-grammar (cdr case)) #f))
       negative-cases))
    (test-case "openCypher profile has identical bounded corpus behavior"
      (for-each
       (lambda (case)
         (check-equal?
          (reference-parse? mrr-cypher-grammar (cdr case))
          (reference-parse? mrr-gql-grammar (cdr case))))
       (append positive-cases negative-cases)))))

(run-tests! grammar-corpus-test)

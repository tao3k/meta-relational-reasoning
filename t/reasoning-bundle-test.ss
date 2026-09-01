#!/usr/bin/env gxi
;;; Executable contracts for Gerbil ReasoningBundle declaration ownership.

(import :std/test
        :clan/poo/object
        :poo-flow/src/core/object-syntax
        :meta-relational-reasoning/scheme/reasoning/core
        :meta-relational-reasoning/scheme/reasoning/default)
(export reasoning-bundle-test)

(include "../scheme/reasoning/declaration.ss")

(with-mrr-reasoning-module
 defmrr-reasoning-module mrr-repeat-reasoning-module)

(def reasoning-bundle-test
  (test-suite "MRR Gerbil reasoning module"
    (test-case "declaration expands into the canonical POO role"
      (check-equal? (.ref mrr-default-reasoning-module 'kind)
                    'mrr-reasoning-module)
      (check-equal? (.ref mrr-default-reasoning-module 'reasoning-module-id)
                    'dependency-closure))
    (test-case "bundle contains every required semantic section"
      (check-equal? (length (.ref mrr-default-reasoning-module
                                  'relation-schemas)) 2)
      (check-equal? (length (.ref mrr-default-reasoning-module
                                  'query-templates)) 1)
      (check-equal? (length (.ref mrr-default-reasoning-module
                                  'rule-packs)) 2)
      (check-equal? (length (.ref mrr-default-reasoning-module
                                  'inverse-goals)) 1)
      (check-equal? (length (.ref mrr-default-reasoning-module
                                  'transition-systems)) 1))
    (test-case "lineage projection and validation policies are explicit"
      (check-equal? (.ref mrr-default-reasoning-module 'lineage-policy)
                    'complete)
      (check-equal? (.ref mrr-default-reasoning-module 'projection-policy)
                    '(#t #t))
      (check-equal? (.ref mrr-default-reasoning-module 'validation-profile)
                    '(64 #t)))
    (test-case "same Scheme module expands byte-for-byte structurally"
      (for-each
       (lambda (slot)
         (check-equal? (.ref mrr-default-reasoning-module slot)
                       (.ref mrr-repeat-reasoning-module slot)))
       '(relation-schemas query-templates rule-packs inverse-goals
         transition-systems lineage-policy projection-policy
         validation-profile)))))

(run-tests! reasoning-bundle-test)

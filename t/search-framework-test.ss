#!/usr/bin/env gxi
;;; Executable contracts for the backend-neutral POO Search framework.

(import :std/test
        :poo-flow/src/core/object-syntax
        :meta-relational-reasoning/scheme/search/core)

(export search-framework-test)

(def consumer-acquisition-role
  (poo-core-role-object
   (slots ((consumer 'fixture)
           (backend-capability 'lexical-candidates)))
   (supers search-acquisition-role)))

(def consumer-refinement-role
  (poo-core-role-object
   (slots ((consumer 'fixture)
           (backend-capability 'rank-candidates)))
   (supers search-refinement-role)))

(def source-stage
  (make-search-stage 'source 'fixture-source '()
                     'workspace 'candidate-set
                     consumer-acquisition-role))

(def rank-stage
  (make-search-stage 'rank 'fixture-rank '()
                     'candidate-set 'ranked-candidate-set
                     consumer-refinement-role))

(def source-rank-strategy
  (make-search-strategy
   'fixture-search
   (make-search-chain 'candidate-ranking (list source-stage rank-stage))
   "runtime-generation-one"
   '((limit . 20) (precision-at-k . required))))

(def (raises? thunk)
  (with-catch
   (lambda (_) #t)
   (lambda () (thunk) #f)))

(def search-framework-test
  (test-suite "backend-neutral POO Search framework"
    (test-case "consumer roles extend the abstract POO stage roles"
      (check-equal? (search-object-ref source-stage 'kind) 'search-stage)
      (check-equal? (search-object-ref source-stage 'search/stage-role)
                    'acquisition)
      (check-equal? (search-object-ref source-stage 'consumer) 'fixture)
      (check-equal? (search-object-ref source-stage 'backend-capability)
                    'lexical-candidates))
    (test-case "factor observations reuse POO Flow temporal causality"
      (let* ((acquired
             (make-search-factor-observation
               source-rank-strategy "runtime-generation-one"
               "event-acquired" "request-one" source-stage "candidate-one"
               1 "runtime-owner-one" '() 'observed #t))
             (refined
              (make-search-factor-observation
               source-rank-strategy "runtime-generation-one"
               "event-refined" "request-one" rank-stage "candidate-one"
               2 "runtime-owner-one" '("event-acquired") 'derived #t))
             (graph
              (make-search-causal-event-graph
               "request-one" (list refined acquired))))
        (check-equal? (search-object-ref acquired 'event-kind) 'source)
        (check-equal? (search-object-ref refined 'event-kind) 'rank)
        (check-equal? (search-object-ref graph 'complete?) #t)))
    (test-case "typed sequential composition produces an inert strategy"
      (let* ((chain (make-search-chain 'candidate-ranking
                                       (list source-stage rank-stage)))
             (strategy (make-search-strategy
                        'fixture-search chain "runtime-generation-one"
                        '((limit . 20) (precision-at-k . required)))))
        (check-equal? (search-node-input-domain chain) 'workspace)
        (check-equal? (search-node-output-domain chain)
                      'ranked-candidate-set)
        (check-equal? (search-object-ref chain 'mode) 'sequential)
        (check-equal? (search-strategy-policy strategy)
                      '((limit . 20) (precision-at-k . required)))
        (check-equal? (search-object-ref strategy 'execution-owner)
                      'consumer-runtime)
        (check (pair? (search-strategy-dag-receipt strategy)) => #t)))
    (test-case "Scheme strategy projects Rust factor identities and generation"
      (check-equal?
       (search-strategy-generation-canonical-input source-rank-strategy)
       "runtime-generation-one")
      (check-equal?
       (search-strategy-factor-rows source-rank-strategy)
       '(("mrr.search.factor.v1:fixture-search:source" "acquisition")
         ("mrr.search.factor.v1:fixture-search:rank" "refinement")))
      (check-equal?
       (search-strategy-factor-edges source-rank-strategy)
       '(("mrr.search.factor.v1:fixture-search:source"
          "mrr.search.factor.v1:fixture-search:rank"))))
    (test-case "stale generation and foreign factors fail closed"
      (check
       (raises? (lambda ()
                  (make-search-factor-observation
                   source-rank-strategy "runtime-generation-two"
                   "stale-event" "request-one" source-stage "candidate-one"
                   1 "runtime-owner-one" '() 'observed #t))) => #t)
      (let (foreign
            (make-search-stage 'foreign 'fixture-foreign '()
                               'workspace 'candidate-set
                               consumer-acquisition-role))
        (check
         (raises? (lambda ()
                    (make-search-factor-observation
                     source-rank-strategy "runtime-generation-one"
                     "foreign-event" "request-one" foreign "candidate-one"
                     1 "runtime-owner-one" '() 'observed #t))) => #t)))
    (test-case "mismatched sequential domains fail closed"
      (let (invalid
            (make-search-stage 'invalid 'fixture-invalid '()
                               'evidence-graph 'public-result
                               search-projection-role))
        (check
         (raises? (lambda ()
                    (make-search-chain 'invalid-chain
                                       (list source-stage invalid)))) => #t)))
    (test-case "parallel branches share one input domain"
      (let ((other
             (make-search-stage 'other 'fixture-other '()
                                'workspace 'structural-candidates
                                consumer-acquisition-role))
            (wrong
             (make-search-stage 'wrong 'fixture-wrong '()
                                'candidate-set 'structural-candidates
                                consumer-acquisition-role)))
        (let (parallel (make-search-parallel 'independent-candidates
                                             (list source-stage other)))
          (check-equal? (search-object-ref parallel 'mode) 'parallel)
          (check-equal? (search-node-input-domain parallel) 'workspace)
          (check-equal?
           (search-strategy-factor-edges
            (make-search-strategy 'parallel-search parallel
                                  "runtime-generation-one" '()))
           '()))
        (check
         (raises? (lambda ()
                    (make-search-parallel 'invalid-parallel
                                          (list source-stage wrong)))) => #t)))
    (test-case "parallel aggregation is an explicit consumer-owned stage"
      (let* ((other
              (make-search-stage 'other 'fixture-other '()
                                 'workspace 'structural-candidates
                                 consumer-acquisition-role))
             (parallel
              (make-search-parallel 'independent-candidates
                                    (list source-stage other)))
             (merge
              (make-search-stage 'merge 'fixture-merge '()
                                 (search-node-output-domain parallel)
                                 'candidate-set
                                 consumer-refinement-role))
             (merged (make-search-merge 'explicit-merge parallel merge)))
        (check-equal? (search-object-ref merged 'mode) 'merge)
        (check-equal? (search-node-input-domain merged) 'workspace)
        (check-equal? (search-node-output-domain merged) 'candidate-set)
        (check-equal?
         (search-strategy-factor-edges
          (make-search-strategy 'merged-search merged
                                "runtime-generation-one" '()))
         '(("mrr.search.factor.v1:merged-search:source"
            "mrr.search.factor.v1:merged-search:merge")
           ("mrr.search.factor.v1:merged-search:other"
            "mrr.search.factor.v1:merged-search:merge")))
        (check
         (raises? (lambda ()
                    (make-search-merge 'invalid-merge parallel rank-stage))) => #t)))))

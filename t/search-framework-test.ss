#!/usr/bin/env gxi
;;; MRR factor projection from POO Flow's typed Search declaration.

(import :std/test
        (only-in :clan/poo/object .ref)
        :poo-flow/src/core/object-syntax
        :poo-flow/modules/search/interface
        (only-in :poo-flow/modules/temporal-causality/interface
                 poo-flow-causal-event-graph)
        :meta-relational-reasoning/scheme/search/projection)

(export search-framework-test)

(def acquisition-role
  (poo-core-role-object
   (slots ((consumer 'fixture)))
   (supers poo-flow-search-acquisition-role)))

(def refinement-role
  (poo-core-role-object
   (slots ((consumer 'fixture)))
   (supers poo-flow-search-refinement-role)))

(def source-stage
  (poo-flow-search-stage 'source 'fixture-source '()
                         'workspace 'candidate-set acquisition-role))

(def rank-stage
  (poo-flow-search-stage 'rank 'fixture-rank '()
                         'candidate-set 'ranked-candidate-set refinement-role))

(def source-rank-strategy
  (poo-flow-search-strategy
   'fixture-search
   (poo-flow-search-chain 'candidate-ranking (list source-stage rank-stage))
   '((limit . 20))))

(def source-rank-projection
  (mrr-search-project source-rank-strategy "runtime-generation-one"))

(def (raises? thunk)
  (with-catch (lambda (_) #t) (lambda () (thunk) #f)))

(def search-framework-test
  (test-suite "MRR projection of POO Flow Search"
    (test-case "POO Flow owns typed composition; MRR projects native identities"
      (check-equal? (.ref source-stage 'search/stage-role) 'acquisition)
      (check-equal? (.ref source-rank-projection 'strategy) source-rank-strategy)
      (check-equal? (.ref source-rank-projection 'generation-canonical-input)
                    "runtime-generation-one")
      (check-equal? (.ref source-rank-projection 'factor-rows)
                    '(("mrr.search.factor.v1:fixture-search:source" "acquisition")
                      ("mrr.search.factor.v1:fixture-search:rank" "refinement")))
      (check-equal? (.ref source-rank-projection 'factor-edges)
                    '(("mrr.search.factor.v1:fixture-search:source"
                       "mrr.search.factor.v1:fixture-search:rank")))
      (check (pair? (.ref source-rank-projection 'dag-receipt)) => #t))
    (test-case "generation and factor membership bind observations"
      (let* ((acquired
              (mrr-search-factor-observation
               source-rank-projection "runtime-generation-one"
               "event-acquired" "request-one" source-stage "candidate-one"
               1 "runtime-owner-one" '() 'observed #t))
             (refined
              (mrr-search-factor-observation
               source-rank-projection "runtime-generation-one"
               "event-refined" "request-one" rank-stage "candidate-one"
               2 "runtime-owner-one" '("event-acquired") 'derived #t))
             (graph (poo-flow-causal-event-graph
                     "request-one" (list refined acquired))))
        (check-equal? (.ref graph 'complete?) #t))
      (check
       (raises? (lambda ()
                  (mrr-search-factor-observation
                   source-rank-projection "runtime-generation-two"
                   "stale" "request-one" source-stage "candidate-one"
                   1 "runtime-owner-one" '() 'observed #t))) => #t)
      (let (foreign
            (poo-flow-search-stage 'foreign 'fixture-foreign '()
                                   'workspace 'candidate-set acquisition-role))
        (check
         (raises? (lambda ()
                    (mrr-search-factor-observation
                     source-rank-projection "runtime-generation-one"
                     "foreign" "request-one" foreign "candidate-one"
                     1 "runtime-owner-one" '() 'observed #t))) => #t)))
    (test-case "parallel branches have no invented influence edge"
      (let* ((other
              (poo-flow-search-stage 'other 'fixture-other '()
                                     'workspace 'structural-candidates
                                     acquisition-role))
             (parallel
              (poo-flow-search-parallel 'parallel-search
                                        (list source-stage other)))
             (strategy (poo-flow-search-strategy 'parallel-search parallel '())))
        (check-equal?
         (.ref (mrr-search-project strategy "runtime-generation-one") 'factor-edges)
         '())))
    (test-case "explicit merge receives both branch edges"
      (let* ((other
              (poo-flow-search-stage 'other 'fixture-other '()
                                     'workspace 'structural-candidates
                                     acquisition-role))
             (parallel
              (poo-flow-search-parallel 'parallel-search
                                        (list source-stage other)))
             (merge-stage
              (poo-flow-search-stage
               'merge 'fixture-merge '()
               (poo-flow-search-node-output-domain parallel)
               'candidate-set refinement-role))
             (merged (poo-flow-search-merge 'explicit-merge parallel merge-stage))
             (strategy (poo-flow-search-strategy 'merged-search merged '())))
        (let (edges (.ref (mrr-search-project strategy "runtime-generation-one")
                          'factor-edges))
          (check-equal? (length edges) 2)
          (check (pair? (member
                         '("mrr.search.factor.v1:merged-search:source"
                           "mrr.search.factor.v1:merged-search:merge") edges)) => #t)
          (check (pair? (member
                         '("mrr.search.factor.v1:merged-search:other"
                           "mrr.search.factor.v1:merged-search:merge") edges)) => #t))))
    (test-case "duplicate factor names fail closed"
      (let* ((duplicate
              (poo-flow-search-stage 'source 'another '()
                                     'candidate-set 'result-set refinement-role))
             (chain (poo-flow-search-chain 'duplicate
                                           (list source-stage duplicate)))
             (strategy (poo-flow-search-strategy 'same-name chain '())))
        (check (raises? (lambda () (mrr-search-project strategy "generation")))
               => #t)))))

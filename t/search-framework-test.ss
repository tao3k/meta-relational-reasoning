#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
;;; MRR factor projection from Agentic AI Lab's typed Search declaration.

(import :std/test
        (only-in :clan/poo/object .ref)
        :yuanxu/object-family/role-syntax
        :agentic-ai-lab/modules/search-engine/interface
        (only-in :agentic-ai-lab/modules/temporal-causality/funs
                 agentic-ai-lab-causal-event-graph)
        :meta-relational-reasoning/scheme/search/projection)

(export search-framework-test)

(def acquisition-role
  (poo-core-role-object
   (slots ((consumer 'fixture)))
   (supers agentic-ai-lab-search-acquisition-role)))

(def refinement-role
  (poo-core-role-object
   (slots ((consumer 'fixture)))
   (supers agentic-ai-lab-search-refinement-role)))

(def source-stage
  (agentic-ai-lab-search-stage 'source 'fixture-source '()
                         'workspace 'candidate-set acquisition-role))

(def rank-stage
  (agentic-ai-lab-search-stage 'rank 'fixture-rank '()
                         'candidate-set 'ranked-candidate-set refinement-role))

(def source-rank-strategy
  (agentic-ai-lab-search-strategy
   'fixture-search
   (agentic-ai-lab-search-chain 'candidate-ranking (list source-stage rank-stage))
   '((limit . 20))))

(def source-rank-projection
  (mrr-search-project source-rank-strategy "runtime-generation-one"))

(def (raises? thunk)
  (with-catch (lambda (_) #t) (lambda () (thunk) #f)))

(def search-framework-test
  (test-suite "MRR projection of Agentic AI Lab Search"
    (test-case "DAG receipt and stage graph disagreement are rejected"
      (let (forged-receipt
            (poo-core-role-object
             (slots ((dag-receipt '((kind . forged)))))
             (supers source-rank-strategy)))
        (check (raises? (lambda () (mrr-search-project forged-receipt "generation"))) => #t))
      ;; Keep the tree intact but compile only its source Flow. The strategy
      ;; owns a genuine receipt for that Flow, so this exercises inventory
      ;; correspondence rather than only the receipt equality check.
      (let* ((root (.ref source-rank-strategy 'root))
             (missing-rank
              (poo-core-role-object
               (slots ((flow (agentic-ai-lab-search-node-flow source-stage))))
               (supers root)))
             (strategy (agentic-ai-lab-search-strategy 'missing-rank missing-rank '())))
        (check (raises? (lambda () (mrr-search-project strategy "generation"))) => #t)))
    (test-case "actual POO strategy directly drives MRR inference"
      (let (answer (mrr-search-evaluate source-rank-projection "runtime-generation-one" (list source-stage)))
        (check-equal? (.ref answer 'projection) source-rank-projection)
        (check (member '(0 "mrr.search.factor.v1:fixture-search:source"
                           "mrr.search.factor.v1:fixture-search:rank" 1)
                       (.ref answer 'influences)) ? pair?))
      (check (raises? (lambda () (mrr-search-evaluate source-rank-projection "stale" (list source-stage)))) => #t)
      (let (foreign (agentic-ai-lab-search-stage 'foreign 'foreign '() 'workspace 'candidate-set acquisition-role))
        (check (raises? (lambda () (mrr-search-evaluate source-rank-projection "runtime-generation-one" (list foreign)))) => #t)))
    (test-case "Agentic AI Lab owns typed composition; MRR projects native identities"
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
             (graph (agentic-ai-lab-causal-event-graph
                     "request-one" (list refined acquired))))
        (check-equal? (.ref graph 'complete?) #t))
      (check
       (raises? (lambda ()
                  (mrr-search-factor-observation
                   source-rank-projection "runtime-generation-two"
                   "stale" "request-one" source-stage "candidate-one"
                   1 "runtime-owner-one" '() 'observed #t))) => #t)
      (let (foreign
            (agentic-ai-lab-search-stage 'foreign 'fixture-foreign '()
                                   'workspace 'candidate-set acquisition-role))
        (check
         (raises? (lambda ()
                    (mrr-search-factor-observation
                     source-rank-projection "runtime-generation-one"
                     "foreign" "request-one" foreign "candidate-one"
                     1 "runtime-owner-one" '() 'observed #t))) => #t)))
    (test-case "parallel branches have no invented influence edge"
      (let* ((other
              (agentic-ai-lab-search-stage 'other 'fixture-other '()
                                     'workspace 'structural-candidates
                                     acquisition-role))
             (parallel
              (agentic-ai-lab-search-parallel 'parallel-search
                                        (list source-stage other)))
             (strategy (agentic-ai-lab-search-strategy 'parallel-search parallel '())))
        (check-equal?
         (.ref (mrr-search-project strategy "runtime-generation-one") 'factor-edges)
         '())))
    (test-case "explicit merge receives both branch edges"
      (let* ((other
              (agentic-ai-lab-search-stage 'other 'fixture-other '()
                                     'workspace 'structural-candidates
                                     acquisition-role))
             (parallel
              (agentic-ai-lab-search-parallel 'parallel-search
                                        (list source-stage other)))
             (merge-stage
              (agentic-ai-lab-search-stage
               'merge 'fixture-merge '()
               (agentic-ai-lab-search-node-output-domain parallel)
               'candidate-set refinement-role))
             (merged (agentic-ai-lab-search-merge 'explicit-merge parallel merge-stage))
             (strategy (agentic-ai-lab-search-strategy 'merged-search merged '())))
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
              (agentic-ai-lab-search-stage 'source 'another '()
                                     'candidate-set 'result-set refinement-role))
             (chain (agentic-ai-lab-search-chain 'duplicate
                                           (list source-stage duplicate)))
             (strategy (agentic-ai-lab-search-strategy 'same-name chain '())))
        (check (raises? (lambda () (mrr-search-project strategy "generation")))
               => #t)))))

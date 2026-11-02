#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Agentic AI Lab is MRR's sole direct Gerbil dependency and owns the parser edge.

(import :std/test
        (only-in :clan/poo/object .o .ref)
        (only-in :clan/poo/mop validate)
        (only-in :agentic-ai-lab/modules/query/types AgenticAILabQuery)
        (only-in :agentic-ai-lab/modules/query/objects
                 AgenticAILabQuery. agentic-ai-lab-query-result-contract
                 agentic-ai-lab-query-element-space)
        (only-in :agentic-ai-lab/modules/query/funs agentic-ai-lab-query-admit))

(def dependencies
  (call-with-input-file "gerbil.pkg"
    (lambda (port)
      (cadr (memq depend: (read port))))))

(def (dependency-count prefix)
  (length (filter (lambda (dependency)
                    (string-prefix? prefix dependency))
                  dependencies)))

(def common-query
  (validate
   AgenticAILabQuery
   (.o (:: @ AgenticAILabQuery.)
       identity: 'mrr/common-query-fixture
       version: "1"
       semantic-revision: "generation-1"
       element-space-identity: 'mrr/element-space-fixture
       selected-element-identities: '(element-1)
       result-contract:
       (agentic-ai-lab-query-result-contract
        'mrr/result-fixture 'relation-row '(identity) 1))))

(def dependency-contract-test
  (test-suite "package dependency ownership"
    (test-case "Agentic AI Lab is the sole direct library dependency"
      (check (length dependencies) => 1)
      (check (dependency-count "github.com/tao3k/agentic-ai-lab@") => 1))
    (test-case "Parser acquisition is inherited; no ASP production dependency is added"
      (check (dependency-count "github.com/tao3k/gerbil-parser@") => 0)
      (check (dependency-count "github.com/tao3k/asp-gerbil-scheme") => 0))
    (test-case "Agentic AI Lab common Query admits a generation-bound library value"
      (let* ((space
              (agentic-ai-lab-query-element-space
               'mrr/element-space-fixture "generation-1" '(element-1) #t))
             (accepted (agentic-ai-lab-query-admit common-query space))
             (stale
              (agentic-ai-lab-query-admit
               common-query
               (agentic-ai-lab-query-element-space
                'mrr/element-space-fixture "generation-2" '(element-1) #t))))
        (check (.ref accepted 'accepted?) => #t)
        (check (.ref accepted 'runtime-executed?) => #f)
        (check (.ref stale 'accepted?) => #f)))))

(export dependency-contract-test)

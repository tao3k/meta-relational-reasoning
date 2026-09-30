#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; POO Flow is MRR's sole direct Gerbil dependency and owns the parser edge.

(import :std/test
        (only-in :clan/poo/object .o .ref)
        (only-in :clan/poo/mop validate)
        :poo-flow/modules/query/interface)

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
   PooFlowQuery
   (.o (:: @ PooFlowQuery.)
       identity: 'mrr/common-query-fixture
       version: "1"
       semantic-revision: "generation-1"
       element-space-identity: 'mrr/element-space-fixture
       selected-element-identities: '(element-1)
       result-contract:
       (poo-flow-query-result-contract
        'mrr/result-fixture 'relation-row '(identity) 1))))

(def dependency-contract-tests
  (test-suite "package dependency ownership"
    (test-case "POO Flow is the sole direct library dependency"
      (check (length dependencies) => 1)
      (check (dependency-count "github.com/tao3k/poo-flow@") => 1))
    (test-case "Parser and ASP acquisition remain inherited from POO Flow"
      (check (dependency-count "github.com/tao3k/gerbil-parser@") => 0)
      (check (dependency-count "github.com/tao3k/asp-gerbil-scheme") => 0))
    (test-case "POO Flow common Query admits a generation-bound library value"
      (let* ((space
              (poo-flow-query-element-space
               'mrr/element-space-fixture "generation-1" '(element-1) #t))
             (accepted (poo-flow-query-admit common-query space))
             (stale
              (poo-flow-query-admit
               common-query
               (poo-flow-query-element-space
                'mrr/element-space-fixture "generation-2" '(element-1) #t))))
        (check (.ref accepted 'accepted?) => #t)
        (check (.ref accepted 'runtime-executed?) => #f)
        (check (.ref stale 'accepted?) => #f)))))

(export dependency-contract-tests)

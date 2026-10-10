;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :std/test check check-exception test-suite test-case)
        (only-in :clan/poo/object .o .ref)
        (only-in :clan/poo/mop validate)
        :poo-flow/modules/query/interface
        :poo-flow/modules/temporal-causality/candidates/interface
        :meta-relational-reasoning/scheme/query/provider/interface)
(export mrr-query-provider-test)

(def QueryResult
  (poo-flow-query-result-contract
   'healthcare/query-result 'relation-row
   '(source target evidence) 32))

(def QuerySpace
  (poo-flow-query-element-space
   'healthcare/case-space "sha256:space-v1"
   '(case-1 profile-1 source-1) #t))

;;; Scheme is the concise host notation.  The value itself is one GQL-aligned
;;; POO AST, so inheritance and slot algebra remain available without creating
;;; a second Scheme Query language.
(def CaseProfileProgram
  (.o (:: @ PooFlowGqlQueryProgram.)
      identity: 'healthcare-case-profile-relations
      match:
      (.o (:: @ GraphSyntaxPath.)
          start: (.o (:: @ GraphSyntaxNode.) binding: 's label: 'Scenario)
          next:
          (.o (:: @ GraphSyntaxStep.)
              relation: 'HAS_CASE
              target: (.o (:: @ GraphSyntaxNode.) binding: 'c label: 'Case)
              next:
              (.o (:: @ GraphSyntaxStep.)
                  relation: 'HAS_EFFECTIVE_PROFILE
                  target:
                  (.o (:: @ GraphSyntaxNode.) binding: 'p label: 'Profile))))
      where:
      (.o (:: @ GraphSyntaxEquals.)
          left:
          (.o (:: @ GraphSyntaxProperty.) binding: 's property: 'identity)
          right:
          (.o (:: @ GraphSyntaxLiteral.)
              literal-kind: 'string value: "healthcare"))
      project:
      (.o (:: @ GraphSyntaxProjection.)
          expression:
          (.o (:: @ GraphSyntaxProperty.) binding: 's property: 'identity)
          next:
          (.o (:: @ GraphSyntaxProjection.)
              expression:
              (.o (:: @ GraphSyntaxProperty.) binding: 'c property: 'id)
              next:
              (.o (:: @ GraphSyntaxProjection.)
                  expression:
                  (.o (:: @ GraphSyntaxProperty.)
                      binding: 'p property: 'identity))))))

(def Query
  (validate
   PooFlowQuery
   (.o (:: @ PooFlowQuery.)
       identity: 'healthcare/case-profile-relations
       version: "1"
       semantic-revision: "sha256:space-v1"
       element-space-identity: 'healthcare/case-space
       selected-element-identities: '(case-1 profile-1)
       language: PooFlowGqlQueryLanguage.
       program: CaseProfileProgram
       result-bound: 16
       completeness-requirement: 'complete
       evidence-requirements: '(provenance-root result-digest)
       visibility-request: 'organization
       result-contract: QueryResult)))

(def (test-sha256 character)
  (string-append "sha256:" (make-string 64 character)))

(def mrr-query-provider-test
  (test-suite "MRR Query Provider"
   (test-case "native MRR admission remains incomplete and separate from scalar rows"
     (let* ((cut (test-sha256 #\a))
            (query
             (validate PooFlowQuery
                       (.o (:: @ Query) semantic-revision: cut)))
            (space
             (poo-flow-query-element-space
              'healthcare/case-space cut
              '(case-1 profile-1 source-1) #t))
            (projection
             (mrr-result-admission-projection
              "native" query (test-sha256 #\b)
              (string-append "mrr:generation:v1:" (make-string 64 #\1))
              (test-sha256 #\c) (test-sha256 #\d) (test-sha256 #\f)
              (test-sha256 #\e) 1 cut 7))
            (admission (poo-flow-query-admit query space))
            (execution-candidate
             (poo-flow-query-execution-candidate
              'mrr (.ref query 'identity) (.ref query 'version) cut
              (poo-flow-query-source-content-identity query)
              'gerbil-parser "sha256:provenance"
              (.ref projection 'result-digest) 1 #t))
            (source-receipt
             (poo-flow-query-bind-execution-receipt
              MrrGqlQueryProvider query admission execution-candidate))
            (scope
             (poo-flow-candidate-scope
              "scope" cut 7 "sha256:coverage" '(gql)))
            (candidate-receipt
             (mrr-projected-candidate-receipt
              "mrr" "candidate" scope query space
              source-receipt projection))
            (exchange
             (poo-flow-candidate-exchange
              "exchange" "candidate" scope (list candidate-receipt)
              (list (poo-flow-candidate-check
                     "claimed-check" candidate-receipt
                     "unverified-native-result" 'valid
                     "sha256:declared-basis")))))
       (check (.ref projection 'native-schema)
              => "mrr.query-result-admission.v1")
       (check (.ref projection 'snapshot-digest) => (test-sha256 #\f))
       (check (.ref projection 'temporal-cut-digest) => cut)
       (check (.ref projection 'temporal-generation) => 7)
       (check (.ref projection 'native-generation)
              => (string-append "mrr:generation:v1:" (make-string 64 #\1)))
       (check-exception
        (mrr-result-admission-projection
         "integer-is-not-native-generation" query (test-sha256 #\b) 7
         (test-sha256 #\c) (test-sha256 #\d) (test-sha256 #\f)
         (test-sha256 #\e) 1 cut 7)
        true)
       (check (.ref candidate-receipt 'complete?) => #f)
       (check (.ref candidate-receipt 'result-digest)
              => (.ref projection 'result-digest))
       (check (.ref exchange 'status) => 'pending)
       (check (.ref exchange 'incomplete-receipts) => '("mrr"))
       (check (.ref exchange 'unchecked-receipts) => '())
       (check (.ref exchange 'admitted?) => #f)
       (check-exception
        (poo-flow-gql-candidate-row-check
         "wrong-profile" candidate-receipt query
         (poo-flow-query-result-set
          "rows" QueryResult (.ref query 'identity)
          (.ref query 'version) cut
          (list (poo-flow-query-result-row
                 "row"
                 (list (poo-flow-query-result-cell 'source "s")
                       (poo-flow-query-result-cell 'target "t")
                       (poo-flow-query-result-cell 'evidence #t)))) #t))
        true)
       (check-exception
        (mrr-projected-candidate-receipt
         "stale" "candidate"
         (poo-flow-candidate-scope
          "scope" cut 8 "sha256:coverage" '(gql))
         query space source-receipt projection)
        true)
       (check-exception
        (mrr-projected-candidate-receipt
         "forged" "candidate" scope query space source-receipt
         (.o (:: @ projection) temporal-generation: 8))
        true)
       (check-exception
        (mrr-result-admission-projection-replay
         (.o (:: @ projection) query-source-digest: (test-sha256 #\0)) query)
        true)
       (check-exception
        (mrr-result-admission-projection-replay
         (.o (:: @ projection) snapshot-digest: (test-sha256 #\0)) query)
        true)
       (check-exception
        (mrr-result-admission-projection-replay
         (.o (:: @ projection)
             native-generation:
             (string-append "mrr:generation:v1:" (make-string 64 #\2))) query)
        true)
       (check-exception
        (mrr-projected-candidate-receipt
         "other-cut" "candidate" scope query space source-receipt
         (mrr-result-admission-projection
          "other" query (test-sha256 #\b)
          (string-append "mrr:generation:v1:" (make-string 64 #\1))
          (test-sha256 #\c) (test-sha256 #\d)
          (test-sha256 #\f) (test-sha256 #\e) 1 (test-sha256 #\0) 7))
        true)
       (check-exception
        (mrr-result-admission-projection
         "bad" query "sha256:short" 7
         (test-sha256 #\c) (test-sha256 #\d) cut
         (test-sha256 #\e) 1 cut 7)
        true)))

))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; Single declarative source for the enhanced Tree-sitter Query V1 extensions.
;;;
;;; This table owns only namespaced #asp-* predicates/directives and their
;;; lowering contract. Ordinary Tree-sitter Query syntax and provider-specific
;;; node/field capability rows remain outside MRR.

(defsyntax (defenhanced-tree-sitter-query-operator-table stx)
  (syntax-case stx
      (profile owner operators recoveries)
    ((_ binding
        (profile profile-id)
        (owner owner-id)
        (operators
         (spelling kind minimum maximum lowering failure-code
                   (position operand-kind domain cardinality) ...) ...)
        (recoveries (site code strategy) ...))
     #'(def binding
         '((profile . profile-id)
           (owner . owner-id)
           (operators
            (spelling kind minimum maximum lowering failure-code
                      ((position operand-kind domain cardinality) ...)) ...)
           (recoveries
            (site code strategy) ...))))))

(defenhanced-tree-sitter-query-operator-table
  mrr-enhanced-tree-sitter-query-operator-table
  (profile "mrr.enhanced-tree-sitter-query.v1")
  (owner "mrr-gerbil-aot")
  (operators
   ("#asp-eq?" predicate 3 3 scalar-eq enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string scalar-fact-path one)
    (2 string literal one))
   ("#asp-not-eq?" predicate 3 3 scalar-not-eq enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string scalar-fact-path one)
    (2 string literal one))
   ("#asp-match?" predicate 3 3 scalar-match enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string scalar-fact-path one)
    (2 string regex one))
   ("#asp-not-match?" predicate 3 3 scalar-not-match enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string scalar-fact-path one)
    (2 string regex one))
   ("#asp-any-eq?" predicate 3 3 set-any-eq enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string set-fact-path one)
    (2 string literal one))
   ("#asp-none-eq?" predicate 3 3 set-none-eq enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string set-fact-path one)
    (2 string literal one))
   ("#asp-any-match?" predicate 3 3 set-any-match enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string set-fact-path one)
    (2 string regex one))
   ("#asp-none-match?" predicate 3 3 set-none-match enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string set-fact-path one)
    (2 string regex one))
   ("#asp-range?" predicate 5 5 range enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string range-fact-path one)
    (2 string range-mode one)
    (3 string canonical-u64 one)
    (4 string canonical-u64 one))
   ("#asp-related?" predicate 3 4 related enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string direction one)
    (2 string relation-kind one)
    (3 string endpoint-selector optional))
   ("#asp-not-related?" predicate 3 4 not-related enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string direction one)
    (2 string relation-kind one)
    (3 string endpoint-selector optional))
   ("#asp-select!" directive 2 unbounded select enhanced-query-operand-invalid
    (0 capture item-capture one)
    (1 string result-field one-or-more)))
  (recoveries
   (operator enhanced-query-operator-unsupported reject)
   (standard-operator enhanced-query-standard-operator-not-resident reject)
   (operand enhanced-query-operand-invalid reject)
   (cardinality enhanced-query-cardinality-invalid reject)
   (capability enhanced-query-capability-row-missing reject)
   (fact enhanced-query-fact-not-resident reject)
   (regex enhanced-query-regex-invalid reject)))

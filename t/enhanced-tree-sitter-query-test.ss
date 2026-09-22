#!/usr/bin/env gxi
;;; Executable contracts for the Scheme-owned enhanced Query V1 declaration.

(import :std/test
        :meta-relational-reasoning/scheme/search/enhanced-tree-sitter-query)

(export enhanced-tree-sitter-query-test)

(def enhanced-tree-sitter-query-test
  (test-suite "enhanced Tree-sitter Query V1 operator table"
    (test-case "closed namespaced table is internally consistent"
      (check-equal?
       (validate-enhanced-tree-sitter-query-operator-table)
       mrr-enhanced-tree-sitter-query-operator-table)
      (check-equal?
       (enhanced-tree-sitter-query-table 'profile)
       "mrr.enhanced-tree-sitter-query.v1")
      (check-equal?
       (enhanced-tree-sitter-query-table 'owner)
       "mrr-gerbil-aot")
      (check-equal?
       (map car (enhanced-tree-sitter-query-table 'operators))
       '("#asp-eq?" "#asp-not-eq?" "#asp-match?" "#asp-not-match?"
         "#asp-any-eq?" "#asp-none-eq?" "#asp-any-match?"
         "#asp-none-match?" "#asp-range?" "#asp-related?"
         "#asp-not-related?" "#asp-select!")))
    (test-case "relation and result operands retain typed cardinality"
      (let* ((operators (enhanced-tree-sitter-query-table 'operators))
             (related (assoc "#asp-related?" operators))
             (select (assoc "#asp-select!" operators)))
        (check-equal? (list-ref related 2) 3)
        (check-equal? (list-ref related 3) 4)
        (check-equal? (list-ref (list-ref related 6) 3)
                      '(3 string endpoint-selector optional))
        (check-equal? (list-ref select 3) 'unbounded)
        (check-equal? (list-ref (list-ref select 6) 1)
                      '(1 string result-field one-or-more))))
    (test-case "all typed failures are rejecting"
      (check-equal?
       (map caddr (enhanced-tree-sitter-query-table 'recoveries))
       '(reject reject reject reject reject reject reject)))))

(export enhanced-tree-sitter-query-test)

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; Gerbil AOT authority for enhanced Tree-sitter Query V1 extensions.
;;;
;;; This module validates the closed declaration itself. It intentionally does
;;; not parse Agent-authored Tree-sitter Query strings; that parser belongs to
;;; the Search compiler and the selected provider's admitted query grammar.

(import :poo-flow/src/utilities/functional)

(export mrr-enhanced-tree-sitter-query-operator-table
        enhanced-tree-sitter-query-table
        validate-enhanced-tree-sitter-query-operator-table)

(include "enhanced-tree-sitter-query-declaration.ss")

(def (enhanced-tree-sitter-query-table key)
  (cdr (assq key mrr-enhanced-tree-sitter-query-operator-table)))

(def (fail message value)
  (error (string-append "invalid enhanced Tree-sitter Query operator table: "
                        message)
         value))

(def (unique? values)
  (= (length values)
     (length
      (poo-flow-fold-left
       (lambda (value seen) (if (member value seen) seen (cons value seen)))
       '()
       values))))

(def (valid-operator-row? row)
  (and (list? row)
       (= (length row) 7)
       (string? (car row))
       (> (string-length (car row)) 5)
       (string=? (substring (car row) 0 5) "#asp-")
       (memq (cadr row) '(predicate directive))
       (integer? (list-ref row 2))
       (> (list-ref row 2) 0)
       (or (integer? (list-ref row 3))
           (eq? (list-ref row 3) 'unbounded))
       (symbol? (list-ref row 4))
       (symbol? (list-ref row 5))
       (pair? (list-ref row 6))))

(def (validate-enhanced-tree-sitter-query-operator-table)
  (let ((operators (enhanced-tree-sitter-query-table 'operators))
        (recoveries (enhanced-tree-sitter-query-table 'recoveries)))
    (unless (and
             (string=?
              (enhanced-tree-sitter-query-table 'profile)
              "mrr.enhanced-tree-sitter-query.v1")
             (string=?
              (enhanced-tree-sitter-query-table 'owner)
              "mrr-gerbil-aot")
             (pair? operators)
             (poo-flow-all? valid-operator-row? operators)
             (unique? (map car operators))
             (pair? recoveries)
             (poo-flow-all?
              (lambda (row)
                (and (list? row)
                     (= (length row) 3)
                     (symbol? (car row))
                     (symbol? (cadr row))
                     (eq? (caddr row) 'reject)))
              recoveries)
             (unique? (map cadr recoveries)))
      (fail "declaration is not closed and internally consistent"
            mrr-enhanced-tree-sitter-query-operator-table)))
  mrr-enhanced-tree-sitter-query-operator-table)

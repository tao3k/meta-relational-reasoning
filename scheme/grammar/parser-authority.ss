;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; -*- Gerbil -*-
;;; Canonical adapter from gerbil-parser language descriptors to MRR projections.

(import :gerbil-parser/src/language/descriptor
        :gerbil-parser/languages/gql/iso-39075-2024/grammar
        (prefix-in :gerbil-parser/languages/cypher/opencypher-2024-1/grammar
                   cypher-))
(export mrr-gql-parser-authority
        mrr-cypher-parser-authority
        parser-authority-ref)

(def (make-parser-authority grammar)
  (list
   (cons 'schema +language-grammar-schema+)
   (cons 'language (language-grammar-language grammar))
   (cons 'version (language-grammar-version grammar))
   (cons 'contract (language-grammar-contract grammar))
   (cons 'grammar (language-grammar-grammar grammar))
   (cons 'ir (language-grammar-ir grammar))))

(def (parser-authority-ref authority key)
  (alet (row (assq key authority))
    (cdr row)))

(def mrr-gql-parser-authority
  (make-parser-authority gql-iso-language-grammar))

(def mrr-cypher-parser-authority
  (make-parser-authority cypher-opencypher-2024-1-language-grammar))

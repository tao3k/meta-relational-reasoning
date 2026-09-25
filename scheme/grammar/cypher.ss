;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; openCypher MRR projection bound to the canonical parser-owned grammar.

(import ./core
        ./gql-declaration
        ./parser-authority)
(export mrr-cypher-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-cypher-grammar open-cypher "openCypher"
 mrr-cypher-parser-authority)

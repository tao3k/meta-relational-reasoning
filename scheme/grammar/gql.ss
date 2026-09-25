;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; ISO GQL MRR projection bound to the canonical parser-owned grammar.

(import ./core
        ./gql-declaration
        ./parser-authority)
(export mrr-gql-grammar)

(with-mrr-gql-declaration
 defmrr-grammar mrr-gql-grammar iso-gql "ISO GQL"
 mrr-gql-parser-authority)

#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; -*- Gerbil -*-
;;; Native MRR library build declaration.

(import (only-in :std/build-script defbuild-script)
        (only-in :asp-gerbil-scheme/building-api
                 asp-gerbil-scheme-package-spec!
                 asp-gerbil-scheme-library-package-prototype))

(def mrr-library-modules
  '("scheme/grammar/parser-authority"
    "scheme/grammar/parser-authority-receipt"
    "scheme/grammar/core"
    "scheme/grammar/gql-declaration"
    "scheme/grammar/gql-profile"
    "scheme/grammar/gql"
    "scheme/grammar/cypher"
    "scheme/reasoning/core"
    "scheme/reasoning/default"
    "scheme/search/enhanced-tree-sitter-query"
    "scheme/search/core"
    "scheme/grammar/native"
    "scheme/grammar/native-program"))

(asp-gerbil-scheme-package-spec!
 (mrr-library-package-spec
  @ asp-gerbil-scheme-library-package-prototype)
 (spec mrr-build-spec)
 (modules mrr-library-modules))

(defbuild-script (mrr-build-spec))

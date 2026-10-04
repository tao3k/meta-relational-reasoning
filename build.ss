#!/usr/bin/env gxi
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
    "scheme/reasoning/finite"
    "scheme/reasoning/finite-native"
    "scheme/search/enhanced-tree-sitter-query"
    "scheme/search/projection"
    "scheme/query/provider/config"
    "scheme/query/provider/contracts"
    "scheme/query/provider/result-types"
    "scheme/query/provider/result-objects"
    "scheme/query/provider/result-funs"
    "scheme/query/provider/candidate"
    "scheme/query/provider/interface"
    "scheme/grammar/native"
    "scheme/grammar/native-program"))

(asp-gerbil-scheme-package-spec!
 (mrr-library-package-spec
  @ asp-gerbil-scheme-library-package-prototype)
 (spec mrr-build-spec)
 (modules mrr-library-modules))

(def mrr-package-build-main
  (let ()
    (defbuild-script (mrr-build-spec))
    main))

(def (main . args)
  (def progress?
    (let* ((raw (getenv "GERBIL_BUILD_VERBOSE" #f))
           (level (and raw (string->number raw))))
      (and (real? level) (> level 0))))
  (when progress?
    (displayln "MRR-PACKAGE build-script entered " args)
    (force-output))
  (apply mrr-package-build-main args)
  ;; Returning from make includes its pending compiler jobs, but not the
  ;; interpreter's later dynamic-module cleanup. Exit status remains required.
  (when progress?
    (displayln "MRR-PACKAGE build-script returned " args)
    (force-output)))

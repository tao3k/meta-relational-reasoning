#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; MRR Scheme library build declaration.

(import (only-in :std/build-script defbuild-script)
        (only-in :asp-gerbil-scheme/building-api
                 asp-gerbil-scheme-package-spec!
                 asp-gerbil-scheme-library-package-prototype))

(def mrr-library-modules
  '("scheme/grammar/parser-authority"
    "scheme/grammar/parser-language"
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
    "scheme/temporal/native"
    "scheme/search/enhanced-tree-sitter-query"
    "scheme/search/projection"
    "scheme/search/controller-bridge"
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
  ;; Phase boundaries are genuine observations even when detailed diagnostics
  ;; are disabled. A returned build is not interpreter cleanup or process exit.
  (displayln "MRR-PACKAGE build-script entered " args)
  (force-output)
  (apply mrr-package-build-main args)
  (displayln "MRR-PACKAGE build-script returned " args)
  (force-output))

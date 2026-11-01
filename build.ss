#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; MRR Scheme library build declaration.

(import (only-in :std/build-script defbuild-script)
        (only-in :std/make pkg-config)
        (only-in :std/string/misc string-trim)
        (only-in :asp-gerbil-scheme/building-api
                 asp-gerbil-scheme-package-spec!
                 asp-gerbil-scheme-library-package-prototype
                 asp-gerbil-scheme-package-native-profile
                 asp-gerbil-scheme-package-pkg-config-libs
                 asp-gerbil-scheme-native-profile-prepare!
                 asp-gerbil-scheme-native-pkg-config-options))

(def mrr-system-libraries '("openssl"))

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
 (modules mrr-library-modules)
 (pkg-config-libs mrr-system-libraries)
 (native-options-resolver
  (lambda () (asp-gerbil-scheme-native-pkg-config-options mrr-system-libraries))))

(def mrr-package-build-main
  (let ()
    (defbuild-script (mrr-build-spec))
    main))

;; Standard pkg-config owns library identity and location. No CI linker flags.
(def (mrr-system-link-inputs output)
  (def libraries (asp-gerbil-scheme-package-pkg-config-libs mrr-library-package-spec))
  ((asp-gerbil-scheme-native-profile-prepare!
    (asp-gerbil-scheme-package-native-profile mrr-library-package-spec)) libraries)
  ;; Resolve all declared inputs before publishing a complete Cargo receipt.
  (def inputs
    (apply append
     (map
      (lambda (library)
        (def directory (string-trim (pkg-config library "--variable=libdir")))
        (when (string-empty? directory) (error "pkg-config has no library directory" library))
        (cons (string-append "cargo:rustc-link-search=native=" directory)
          (map
           (lambda (option)
             (unless (and (string-prefix? "-l" option) (> (string-length option) 2))
               (error "unsupported pkg-config library name" library option))
             (string-append "cargo:rustc-link-lib=" (substring option 2 (string-length option))))
           (filter (lambda (option) (not (string-empty? option)))
                   (string-split (string-trim (pkg-config library "--libs-only-l")) #\space)))))
      libraries)))
  (def pending (string-append output ".pending"))
  (call-with-output-file pending
    (lambda (port)
      (for-each (lambda (input) (display input port) (newline port)) inputs)))
  (rename-file pending output))

(def (main . args)
  (match args
    (["link-inputs" output] (mrr-system-link-inputs output))
    (else
     ;; Return is not interpreter quiescence or successful process exit.
     (displayln "MRR-PACKAGE build-script entered " args)
     (force-output)
     (apply mrr-package-build-main args)
     (displayln "MRR-PACKAGE build-script returned " args)
     (force-output))))

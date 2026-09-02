#!/usr/bin/env gxi
;;; Canonical GSLPH Building API entrypoint for MRR Gerbil native AOT artifacts.

(import (only-in :std/cli/multicall define-entry-point define-multicall-main)
        :gslph/src/build-api/framework
        (only-in :gerbil/gambit copy-file delete-file exit file-exists? write))

(def +mrr-native-aot-file+
  "meta-relational-reasoning__scheme__grammar__native.scm")

(def (mrr-grammar-staged-native-aot)
  (string-append "scheme/generated/" +mrr-native-aot-file+))

(def (mrr-native-aot-stages)
  (list
   (make-package-source-stage
    "grammar-aot"
    "."
    "meta-relational-reasoning"
    '("scheme/grammar/core"
      "scheme/grammar/gql-declaration"
      "scheme/grammar/gql-profile"
      "scheme/grammar/gql"
      "scheme/grammar/cypher"
      "scheme/reasoning/core"
      "scheme/reasoning/default"
      "scheme/grammar/native")
    #t)))

(def (mrr-stage-native-aot!)
  (let* ((source (string-append ".gerbil/lib/static/" +mrr-native-aot-file+))
         (target (mrr-grammar-staged-native-aot)))
    (unless (file-exists? source)
      (error "GSLPH compile completed without the native AOT artifact" source))
    (when (file-exists? target) (delete-file target))
    (copy-file source target)
    target))

(define-multicall-main)

(define-entry-point (meta)
  (help: "List MRR Gerbil native build targets" getopt: [])
  (write '(spec compile clean))
  (newline))

(define-entry-point (spec)
  (help: "Print the declarative GSLPH native build spec" getopt: [])
  (write (package-source-stages-spec (mrr-native-aot-stages)))
  (newline))

(define-entry-point (compile)
  (help: "Compile and stage the MRR Gerbil native AOT artifact" getopt: [])
  (package-source-stages-run! (mrr-native-aot-stages) [])
  (mrr-stage-native-aot!)
  (exit 0))

(define-entry-point (clean)
  (help: "Clean MRR Gerbil native build artifacts" getopt: [])
  (package-source-stages-clean! (mrr-native-aot-stages))
  (when (file-exists? (mrr-grammar-staged-native-aot))
    (delete-file (mrr-grammar-staged-native-aot)))
  (exit 0))

#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Native MRR library build declaration.

(import :clan/building
        (only-in :asp-gerbil-scheme/src/build-api/package-spec
                 asp-gerbil-scheme-package-spec!
                 asp-gerbil-scheme-library-package-prototype
                 asp-gerbil-scheme-package-build-profile
                 asp-gerbil-scheme-package-native-spec)
        (only-in :asp-gerbil-scheme/src/building/build-script
                 defbuild-script
                 framework-build-bindir))

(asp-gerbil-scheme-package-spec!
 (mrr-library-package-spec
  @ asp-gerbil-scheme-library-package-prototype)
 (role 'library)
 (profile 'development)
 (native-spec
  '("scheme/grammar/core"
    "scheme/grammar/gql-declaration"
    "scheme/grammar/gql-profile"
    "scheme/grammar/gql"
    "scheme/grammar/cypher"
    "scheme/reasoning/core"
    "scheme/reasoning/default"
    "scheme/grammar/native")))

(defbuild-script
 (asp-gerbil-scheme-package-native-spec
  mrr-library-package-spec)
 profile: (asp-gerbil-scheme-package-build-profile
           mrr-library-package-spec)
 bindir: (framework-build-bindir))

;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0
;;; MRR selects language packs; parser owns handles, parsing and result buffers.
(import :gerbil-parser/src/ffi/language-abi
        (only-in :gerbil-parser/src/ffi/language-handles register-language!)
        (only-in :gerbil-parser/languages/gql/parser gql-language-grammar)
        (only-in :gerbil-parser/languages/cypher/parser opencypher-language-grammar))
(export)
(def language-handles (make-vector 2 #f))
(def (language-handle kind)
  (unless (and (fixnum? kind) (<= 0 kind) (< kind 2))
    (error "unknown MRR parser language" kind))
  (or (vector-ref language-handles kind)
      (let (handle (register-language!
                    (if (= kind 0) gql-language-grammar opencypher-language-grammar)))
        (vector-set! language-handles kind handle)
        handle)))
(begin-foreign
  (namespace ("meta-relational-reasoning/scheme/grammar/parser-language#"
              mrr-parser-language-handle))
  (c-define (mrr-parser-language-handle kind) (int32) unsigned-int64
    "mrr_parser_language_handle" "extern"
    (with-exception-catcher (lambda (_) 0)
      (lambda () (meta-relational-reasoning/scheme/grammar/parser-language#language-handle kind)))))

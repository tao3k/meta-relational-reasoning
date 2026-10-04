;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Structural projection only: the native Rust receipt is not verified here.
(import (only-in :clan/poo/object .ref)
        (only-in :std/crypto/digest sha256)
        (only-in :std/encoding/hex hex-encode)
        (only-in :std/list/list every)
        (only-in :poo-flow/modules/query/types poo-flow-query?)
        (only-in :poo-flow/modules/query/contracts
                 poo-flow-query-source-content-identity)
        (only-in "result-types.ss"
                 mrr-result-admission-projection?)
        (only-in "result-objects.ss"
                 mrr-result-admission-projection-value))
(export mrr-result-admission-projection
        mrr-result-admission-projection-replay)

(def +mrr-result-schema+ "mrr.query-result-admission.v1")
(def +mrr-result-prefix+ "mrr.query-result-admission.v1:")
(def (text? value) (and (string? value) (> (string-length value) 0)))
(def (hex-char? character)
  (or (char<=? #\0 character #\9)
      (char<=? #\a character #\f)))
(def (sha256-text? value)
  (and (string? value) (= (string-length value) 71)
       (string=? (substring value 0 7) "sha256:")
       (every hex-char? (string->list (substring value 7 71)))))
(def (native-generation? value)
  (let (prefix "mrr:generation:v1:")
    (and (string? value)
         (= (string-length value) (+ (string-length prefix) 64))
         (string=? (substring value 0 (string-length prefix)) prefix)
         (every hex-char?
                (string->list (substring value (string-length prefix)
                                         (string-length value)))))))
(def (digest datum)
  (string-append
   "sha256:"
   (hex-encode
    (sha256 (string->utf8
             (call-with-output-string
              (lambda (port) (write datum port))))))))

(def (mrr-result-admission-projection
      id query binding generation relation entity snapshot native-result count
      temporal-cut temporal-generation)
  (unless (and (text? id) (poo-flow-query? query)
               (every sha256-text?
                      (list binding relation entity snapshot native-result temporal-cut))
               (native-generation? generation)
               (exact-integer? temporal-generation) (>= temporal-generation 0)
               (exact-integer? count) (>= count 0))
    (error "invalid native MRR result admission projection"))
  (let* ((source (poo-flow-query-source-content-identity query))
         (result (string-append +mrr-result-prefix+ native-result))
         (semantic
          (digest (list 'poo-flow.mrr-result-projection.v2 id
                        +mrr-result-schema+ source binding generation
                        relation entity snapshot result count
                        temporal-cut temporal-generation))))
    (mrr-result-admission-projection-value
     id semantic +mrr-result-schema+ source binding generation
     relation entity snapshot temporal-cut temporal-generation result count)))

(def (mrr-result-admission-projection-replay projection query)
  (unless (and (mrr-result-admission-projection? projection)
               (poo-flow-query? query)
               (equal? (.ref projection 'native-schema)
                       +mrr-result-schema+)
               (string? (.ref projection 'result-digest))
               (> (string-length (.ref projection 'result-digest))
                  (string-length +mrr-result-prefix+))
               (string=?
                (substring (.ref projection 'result-digest)
                           0 (string-length +mrr-result-prefix+))
                +mrr-result-prefix+))
    (error "invalid native MRR projection replay"))
  (let (replayed
        (mrr-result-admission-projection
         (.ref projection 'identity) query
         (.ref projection 'query-binding-digest)
         (.ref projection 'native-generation)
         (.ref projection 'relation-catalog-digest)
         (.ref projection 'entity-catalog-digest)
         (.ref projection 'snapshot-digest)
         (substring (.ref projection 'result-digest)
                    (string-length +mrr-result-prefix+)
                    (string-length (.ref projection 'result-digest)))
         (.ref projection 'result-count)
         (.ref projection 'temporal-cut-digest)
         (.ref projection 'temporal-generation)))
    (unless (and (equal? (.ref projection 'semantic-digest)
                         (.ref replayed 'semantic-digest))
                 (equal? (.ref projection 'query-source-digest)
                         (.ref replayed 'query-source-digest)))
      (error "native MRR projection digest mismatch"))
    replayed))

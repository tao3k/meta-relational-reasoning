;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0
;;; Embedded owner projection; uses MRR's existing Gambit initializer and thread.
(import (only-in ../search/projection mrr-search-compile)
        (only-in :poo-flow/src/ffi/scheme-wire scheme-wire-read scheme-wire-write)
        (only-in :poo-flow/src/ffi/temporal-proof temporal-derivation-admit)
        (only-in :poo-flow/src/ffi/temporal-policy temporal-policy-refresh)
        (only-in :poo-flow/src/ffi/temporal-proof-host
                 temporal-proof-state-refresh temporal-proof-register temporal-proof-current))
(def result-bytes #f)
(def (reset!) (set! result-bytes #f) 0)
(def (invoke operation payload)
  (reset!)
  (with-catch (lambda (_) (reset!) 4)
    (lambda ()
      (unless (<= (u8vector-length (string->utf8 payload)) 1048576) (error "Temporal input bound"))
      (let* ((request (scheme-wire-read payload))
             (value (case operation
                      ((5) (mrr-search-compile payload))
                      ((0) (temporal-policy-refresh request))
                      ((1) (temporal-proof-state-refresh request))
                      ((2) (temporal-proof-register request))
                      ((3) (temporal-derivation-admit request))
                      ((4) (temporal-proof-current request))
                      (else (error "unsupported Temporal operation"))))
             (bytes (string->utf8 (scheme-wire-write value))))
        (unless (<= (u8vector-length bytes) 1048576) (error "Temporal output bound"))
        (set! result-bytes bytes) 0))))
(def (size) (if result-bytes (u8vector-length result-bytes) -1))
(def (byte-at index)
  (if (and result-bytes (<= 0 index) (< index (u8vector-length result-bytes)))
    (u8vector-ref result-bytes index) -1))
(begin-foreign
 (namespace ("meta-relational-reasoning/scheme/temporal/native#"
              invoke reset! size byte-at version native-call native-size native-byte native-reset))
 (c-define (version) () unsigned-int32 "mrr_temporal_abi_version" "extern" 1)
 (c-define (native-call operation payload) (int32 UTF-8-string) int32 "mrr_temporal_call" "extern" (invoke operation payload))
 (c-define (native-size) () int64 "mrr_temporal_result_size" "extern" (size))
 (c-define (native-byte index) (int64) int32 "mrr_temporal_result_byte" "extern" (byte-at index))
 (c-define (native-reset) () int32 "mrr_temporal_reset" "extern" (reset!)))

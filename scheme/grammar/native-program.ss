;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; Gerbil compiler-owned AOT root for the embedded MRR runtime.

(import ./native
        :gerbil-parser/src/ffi/parse-artifact-v1-native)
(export main)

;; The compiler-generated startup stub initializes the complete immutable
;; module closure. Rust calls the exported C ABIs after setup completes.
(def (main . _arguments) (void))

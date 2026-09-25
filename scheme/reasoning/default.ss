;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; Canonical MRR reasoning module instantiated from one declaration.

(import ./core)
(export mrr-default-reasoning-module)

(include "declaration.ss")

(with-mrr-reasoning-module
 defmrr-reasoning-module mrr-default-reasoning-module)

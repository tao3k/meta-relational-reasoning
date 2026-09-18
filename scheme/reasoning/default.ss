;;; Canonical MRR reasoning module instantiated from one declaration.

(import ./core)
(export mrr-default-reasoning-module)

(include "declaration.ss")

(with-mrr-reasoning-module
 defmrr-reasoning-module mrr-default-reasoning-module)

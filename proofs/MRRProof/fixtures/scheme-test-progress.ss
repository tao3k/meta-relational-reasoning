;;; Report actual expander work while upstream gxtest prepares its harness.
;;; Load this file before importing gxtest; retain both upstream callbacks.
(def mrr-original-import (gx#current-expander-module-import))
(def mrr-original-eval (gx#current-expander-module-eval))
(def mrr-reported-imports (make-hash-table))
(def (mrr-test-progress stage path)
  (displayln "mrr-test: " stage " " path)
  (force-output))
(gx#current-expander-module-import
 (lambda (path reload?)
   (let (report? (not (hash-get mrr-reported-imports path)))
     (when report? (mrr-test-progress "import started" path))
     (let (context (mrr-original-import path reload?))
       (when report?
         (hash-put! mrr-reported-imports path #t)
         (mrr-test-progress "import returned" path))
       context))))
(gx#current-expander-module-eval
 (lambda (context)
   (mrr-test-progress "evaluation started" (gx#module-context-id context))
   (let (result (mrr-original-eval context))
     (mrr-test-progress "evaluation returned" (gx#module-context-id context))
     result)))
;;; The compiled runtime loads nested modules while the source evaluator is quiet.
;;; Observe that same loader without changing its resolution or admission rules.
(def mrr-original-load-module load-module)
(def mrr-reported-loads (make-hash-table))
(set! load-module
 (lambda (path)
   (let (report? (not (hash-get mrr-reported-loads path)))
     (when report? (mrr-test-progress "runtime load started" path))
     (let (result (mrr-original-load-module path))
       (when report?
         (hash-put! mrr-reported-loads path #t)
         (mrr-test-progress "runtime load returned" path))
       result))))

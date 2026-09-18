#!/usr/bin/env gxi
;;; Executable contracts for the Scheme-owned outer reasoning loop.

(import :std/test
        :clan/poo/object
        :meta-relational-reasoning/scheme/reasoning/core
        :meta-relational-reasoning/scheme/reasoning/default)
(export reasoning-driver-test)

(def (raises? thunk)
  (with-catch
   (lambda (_) #t)
   (lambda () (thunk) #f)))

(def reasoning-driver-test
  (test-suite "MRR Scheme reasoning driver"
    (test-case "Scheme schedules model then authoritative MRR closure"
      (let* ((initial (mrr-driver-start
                       mrr-default-reasoning-module
                       'task 'visible-world 2))
             (model-request (mrr-driver-request initial))
             (proposed
              (mrr-driver-accept
               initial
               (mrr-resource-receipt
                'model-proposal 'candidate 0 'candidate-bundle)))
             (kernel-request (mrr-driver-request proposed))
             (complete
              (mrr-driver-accept
               proposed
               (mrr-resource-receipt
                'mrr-closure 'admitted 0 'closure-receipt))))
        (check-equal? (mrr-resource-ref model-request 'resource)
                      'model-proposal)
        (check-equal? (mrr-resource-ref kernel-request 'resource)
                      'mrr-closure)
        (check-equal? (mrr-driver-result complete) 'closure-receipt)))
    (test-case "rejected closure returns to proposal within budget"
      (let* ((initial (mrr-driver-start
                       mrr-default-reasoning-module 'task 'world 2))
             (proposed
              (mrr-driver-accept
               initial
               (mrr-resource-receipt
                'model-proposal 'candidate 0 'candidate-bundle)))
             (retry
              (mrr-driver-accept
               proposed
               (mrr-resource-receipt
                'mrr-closure 'rejected 0 'counter-evidence))))
        (check-equal? (mrr-resource-ref (mrr-driver-request retry) 'resource)
                      'model-proposal)
        (check-equal? (mrr-resource-ref (mrr-driver-request retry) 'cycle) 1)))
    (test-case "mismatched authority and budget exhaustion fail closed"
      (let ((initial (mrr-driver-start
                      mrr-default-reasoning-module 'task 'world 1)))
        (check-equal?
         (raises?
          (lambda ()
            (mrr-driver-accept
             initial
             (mrr-resource-receipt
              'mrr-closure 'candidate 0 'forged))))
         #t)
        (let ((proposed
               (mrr-driver-accept
                initial
                (mrr-resource-receipt
                 'model-proposal 'candidate 0 'candidate-bundle))))
          (check-equal?
           (raises?
            (lambda ()
              (mrr-driver-accept
               proposed
               (mrr-resource-receipt
                'mrr-closure 'rejected 0 'counter-evidence))))
           #t))))))

(run-tests! reasoning-driver-test)

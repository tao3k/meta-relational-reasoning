(import :std/test)
(export harness-assertion-test)
(def harness-assertion-test
  (test-suite "qualification assertion failure"
    (test-case "intentional failed assertion" (check-equal? 1 2))))

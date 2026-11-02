(import :std/test)
(export harness-pass-test)
(def harness-pass-test
  (test-suite "qualification success"
    (test-case "known successful assertion" (check-equal? 1 1))))

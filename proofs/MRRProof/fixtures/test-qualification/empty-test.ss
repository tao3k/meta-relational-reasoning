(import :std/test)
(export deliberately-undiscovered-tests)
(def deliberately-undiscovered-tests
  (test-suite "undiscovered export"
    (test-case "must not count as discovered" (check-equal? 1 1))))

#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0

(import :std/test (only-in :std/error Error? Error-message)
        (only-in :clan/poo/object .ref)
        :meta-relational-reasoning/scheme/reasoning/finite)
(export finite-inference-test)

(def (foreign-coordinate-error? value)
  (and (Error? value)
       (equal? (Error-message value) "foreign finite node coordinate")))

(def finite-inference-test
  (test-suite "Scheme-owned finite inference"
    (test-case "only stabilized shortest distances reach observations"
      (let (answer
            (mrr-finite-evaluate
             (mrr-finite-request 4 '((0 1) (1 2) (2 3) (0 3)) '(0))))
        (check (member '(0 3 1) (.ref answer 'paths)) ? pair?)
        (check (member '(0 0 3 1) (.ref answer 'influences)) ? pair?)
        (check (member '(0 0 3 3) (.ref answer 'influences)) => #f)))
    (test-case "cycles have finite positive support and empty requests stay empty"
      (let (answer (mrr-finite-evaluate
                    (mrr-finite-request 2 '((0 1) (1 0)) [])))
        (check (member '(0 0 2) (.ref answer 'paths)) ? pair?))
      (let (answer (mrr-finite-evaluate (mrr-finite-request 0 [] [])))
        (check (.ref answer 'paths) => [])
        (check (.ref answer 'influences) => [])))
    (test-case "foreign coordinates reject before inference"
      (check-exception (mrr-finite-request 2 '((0 2)) []) foreign-coordinate-error?)
      (check-exception (mrr-finite-request 2 [] '(2)) foreign-coordinate-error?))
    (test-case "request owns its source coordinates"
      (let* ((source (list (list 0 1)))
             (request (mrr-finite-request 2 source [])))
        (set-car! (car source) 1)
        (check (.ref request 'edges) => '((0 1)))
        (check (.ref request 'node-count) => 2)))))

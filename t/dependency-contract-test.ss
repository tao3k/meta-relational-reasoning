#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: AGPL-3.0-only

;;; -*- Gerbil -*-
;;; MRR owns only the parser edge; POO Flow and ASP remain transitive.

(import :std/test)

(def dependencies
  (call-with-input-file "gerbil.pkg"
    (lambda (port)
      (cadr (memq depend: (read port))))))

(def (dependency-count prefix)
  (length (filter (lambda (dependency)
                    (string-prefix? prefix dependency))
                  dependencies)))

(def dependency-contract-tests
  (test-suite "package dependency ownership"
    (test-case "Gerbil Parser is the sole direct project dependency"
      (check (length dependencies) => 1)
      (check (dependency-count "github.com/tao3k/gerbil-parser@") => 1))
    (test-case "POO Flow and ASP acquisition remain inherited from Gerbil Parser"
      (check (dependency-count "github.com/tao3k/poo-flow") => 0)
      (check (dependency-count "github.com/tao3k/asp-gerbil-scheme") => 0))))

(export dependency-contract-tests)

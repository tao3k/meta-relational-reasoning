;;; -*- Gerbil -*-
;;; Fixed-width native receipt for the linked parser authority.
;;;
;;; grammar-authority-test.ss compares every value against the live parser
;;; descriptor, so dependency drift fails before this receipt reaches Rust.

(export mrr-gql-parser-authority-receipt)

(include "parser-authority-receipt-declaration.ss")

;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0

;;; POO-native composition of ASCENT's maintained unit-weight path lattice.
(import (only-in :clan/poo/object .o .ref .call .mix)
        (only-in :clan/poo/trie UIntTrieSet)
        (only-in :gerbil-ascent/table/expression
                 gerbil-ascent-table-expression-prototype))
(export mrr-finite-request mrr-finite-evaluate)

(def (mrr-finite-request node-limit source-edges observation-factors)
  (unless (and (exact-integer? node-limit) (>= node-limit 0)
               (list? source-edges) (list? observation-factors))
    (error "invalid finite request"))
  (def (node? value)
    (and (exact-integer? value) (<= 0 value) (< value node-limit)))
  (unless (and (andmap (lambda (row)
                        (and (list? row) (= (length row) 2)
                             (andmap node? row))) source-edges)
               (andmap node? observation-factors))
    (error "foreign finite node coordinate"))
  ;; Copy before creating lazy POO slots: a delayed map would still observe
  ;; subsequent mutation of the caller's input buffers on first slot access.
  (let ((owned-edges
         (map (lambda (row) (list (car row) (cadr row))) source-edges))
        (owned-observations (map (lambda (node) node) observation-factors)))
    (.o (kind 'mrr-finite-request)
        (node-count node-limit)
        (edges owned-edges) (observations owned-observations))))

(def (mrr-finite-evaluate supplied)
  (let* ((bound-request (mrr-finite-request (.ref supplied 'node-count)
                                    (.ref supplied 'edges)
                                    (.ref supplied 'observations)))
         (nodes (.ref bound-request 'node-count))
         (wire-radix (max 2 nodes))
         (source
          (.call UIntTrieSet .<-list
                 (map (lambda (edge) (+ (* (car edge) wire-radix) (cadr edge)))
                      (.ref bound-request 'edges))))
         (expression
          (.mix gerbil-ascent-table-expression-prototype
                (.o (source-pairs source) (radix wire-radix))))
         ;; The upstream bounded closure and lattice independently publish
         ;; the same finite pair inventory. Distances stay ASCENT-owned.
         (closure ((.ref expression 'closure-bounded) (max 1 (* nodes nodes))))
         (pairs (.ref expression 'shortest-distance-pairs))
         (distance-of (.ref expression 'shortest-distance-of)))
    (unless (equal? pairs (.ref closure 'pairs))
      (error "ASCENT bounded closure and path lattice disagree"))
    (let* ((path-rows
            (map (lambda (pair)
                   (list (quotient pair wire-radix) (modulo pair wire-radix)
                         (distance-of pair))) pairs))
           (observation-nodes (.ref bound-request 'observations))
           ;; This is a value projection over the stabilized upstream lattice,
           ;; preserving original observation order and self influence at zero.
           (influence-rows
            (apply append
                   (map
                    (lambda (index from)
                      (cons (list index from from 0)
                            (map (lambda (path)
                                   (list index from (cadr path) (caddr path)))
                                 (filter (lambda (path)
                                           (and (= from (car path))
                                                (not (= from (cadr path)))))
                                         path-rows))))
                    (iota (length observation-nodes)) observation-nodes))))
      (.o (kind 'mrr-finite-candidate)
          (request bound-request) (paths path-rows) (influences influence-rows)))))

;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0

;;; Internal fixed-width projection, used only on the Gambit owner thread.
(import ./finite (only-in :clan/poo/object .ref))

(def native-nodes #f)
(def native-edge-limit 0)
(def native-observation-limit 0)
(def native-edges [])
(def native-observations [])
(def native-result #f)
(def native-edge-count 0)
(def native-observation-count 0)
(def native-paths #f)
(def native-influences #f)

(def (reset!)
  (set! native-nodes #f)
  (set! native-edges [])
  (set! native-observations [])
  (set! native-result #f)
  (set! native-edge-count 0)
  (set! native-observation-count 0)
  (set! native-paths #f)
  (set! native-influences #f)
  0)
(def (start! nodes edge-limit observation-limit)
  (reset!)
  (if (and (>= nodes 0) (>= edge-limit 0) (>= observation-limit 0))
    (begin (set! native-nodes nodes)
           (set! native-edge-limit edge-limit)
           (set! native-observation-limit observation-limit) 0)
    1))
(def (node? node)
  (and native-nodes (>= node 0) (< node native-nodes)))
(def (edge! from to)
  (if (and (node? from) (node? to)
           (< native-edge-count native-edge-limit) (not native-result))
    (begin (set! native-edges (cons (list from to) native-edges))
           (set! native-edge-count (+ native-edge-count 1)) 0)
    1))
(def (observe! factor)
  (if (and (node? factor)
           (< native-observation-count native-observation-limit)
           (not native-result))
    (begin (set! native-observations (cons factor native-observations))
           (set! native-observation-count (+ native-observation-count 1)) 0)
    1))
(def (solve!)
  (with-catch
   (lambda (_) (reset!) 2)
   (lambda ()
     (unless native-nodes (error "no native finite request"))
     (set! native-result
       (mrr-finite-evaluate
        (mrr-finite-request native-nodes (reverse native-edges)
                            (reverse native-observations))))
     (set! native-paths
       (list->vector (map list->vector (.ref native-result 'paths))))
     (set! native-influences
       (list->vector (map list->vector (.ref native-result 'influences))))
     0)))
(def (rows table)
  (and native-result
       (case table
         ((0) native-paths)
         ((1) native-influences)
         (else #f))))
(def (count-rows table)
  (let (values (rows table)) (if values (vector-length values) -1)))
(def (cell table row column)
  (let (values (rows table))
    (if (and values (<= 0 row) (< row (vector-length values)))
      (let (value (vector-ref values row))
        (if (and (<= 0 column) (< column (vector-length value)))
          (vector-ref value column) -1))
      -1)))

(begin-foreign
 (namespace
  ("meta-relational-reasoning/scheme/reasoning/finite-native#"
   start! edge! observe! solve! reset! count-rows cell
   finite-version finite-start finite-edge finite-observe finite-solve
   finite-reset finite-count finite-cell))
 (c-define (finite-version) () unsigned-int32 "mrr_finite_abi_version" "extern" 1)
 (c-define (finite-start nodes edges observations)
   (int64 int64 int64) int32 "mrr_finite_start" "extern"
   (start! nodes edges observations))
 (c-define (finite-edge from to) (int64 int64) int32 "mrr_finite_edge" "extern"
   (edge! from to))
 (c-define (finite-observe factor) (int64) int32 "mrr_finite_observe" "extern"
   (observe! factor))
 (c-define (finite-solve) () int32 "mrr_finite_solve" "extern" (solve!))
 (c-define (finite-reset) () int32 "mrr_finite_reset" "extern" (reset!))
 (c-define (finite-count table) (int32) int64 "mrr_finite_count" "extern"
   (count-rows table))
 (c-define (finite-cell table row column)
   (int32 int64 int64) int64 "mrr_finite_cell" "extern"
   (cell table row column)))

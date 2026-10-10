;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; MRR factor identities projected from a POO Flow Search strategy.
;;; The POO Flow strategy remains the authority for typed composition.

(import (only-in :clan/poo/object .ref object? object-supers compute-precedence-list!)
        (only-in :std/list/list append-map delete-duplicates/hash every filter)
        :poo-flow/src/core/object-syntax
        :poo-flow/modules/search-engine/interface
        (only-in :poo-flow/src/core/plan flow->linear-plan execution-plan-nodes
                 execution-plan->dag-receipt plan-node-id plan-node-kind
                 plan-node-name plan-node-step plan-node-dependencies)
        ../reasoning/finite
        (only-in :poo-flow/src/ffi/scheme-wire scheme-wire-read scheme-wire-write))

(export mrr-search-project
        mrr-search-projection?
        mrr-search-factor-observation
        mrr-search-evaluate
        mrr-search-compile mrr-search-build-strategy mrr-search-compile-strategy)

(def (mrr-search-projection? value)
  (and (object? value)
       (with-catch
        (lambda (_) #f)
        (lambda () (eq? (.ref value 'kind) 'mrr-search-projection)))))

(def (mrr-search-stages node)
  (if (poo-flow-search-stage? node)
    (list node)
    (append-map mrr-search-stages (.ref node 'children))))

(def (mrr-search-boundary node side)
  (if (poo-flow-search-stage? node)
    (list node)
    (let (children (.ref node 'children))
      (if (eq? (.ref node 'mode) 'parallel)
        (append-map (lambda (child) (mrr-search-boundary child side)) children)
        (mrr-search-boundary
         (if (eq? side 'source) (car children) (car (reverse children)))
         side)))))

(def (mrr-search-factor-input strategy-name stage)
  (string-append "mrr.search.factor.v1:"
                 (symbol->string strategy-name) ":"
                 (symbol->string (.ref stage 'name))))

(def (mrr-search-edges strategy-name node)
  (if (poo-flow-search-stage? node)
    '()
    (let* ((children (.ref node 'children))
           (nested (append-map (lambda (child) (mrr-search-edges strategy-name child))
                               children)))
      (if (eq? (.ref node 'mode) 'parallel)
        nested
        (append
         nested
         (append-map
          (lambda (pair)
            (append-map
             (lambda (left)
               (map (lambda (right)
                      (list (mrr-search-factor-input strategy-name left)
                            (mrr-search-factor-input strategy-name right)))
                    (mrr-search-boundary (cadr pair) 'source)))
             (mrr-search-boundary (car pair) 'sink)))
          (let loop ((rest children))
            (if (or (null? rest) (null? (cdr rest)))
              '()
              (cons (list (car rest) (cadr rest)) (loop (cdr rest)))))))))))


;;; Expand compiled branch-arm Flow nodes, contract administrative joins, and
;;; retain only external stage nodes. This reads compiled dependencies rather
;;; than the Search composition tree used by mrr-search-edges.
(def (mrr-search-plan-graph strategy-name flow)
  (let* ((plan (flow->linear-plan flow))
         (nodes (execution-plan-nodes plan))
         (boundaries '()) (factors '()) (edges '()))
    (for-each
     (lambda (node)
       (let* ((id (plan-node-id node))
              (kind (plan-node-kind node))
              (previous
               (delete-duplicates/hash
                (append-map
                 (lambda (dependency)
                   (let (entry (assoc dependency boundaries))
                     (unless entry (error "foreign or forward POO DAG dependency" dependency))
                     (cdr entry)))
                 (plan-node-dependencies node)))))
         (when (assoc id boundaries) (error "duplicate POO DAG node" id))
         (cond
          ((eq? kind 'branch)
           (set! boundaries (cons (cons id previous) boundaries)))
          ((memq kind '(branch-left branch-right flow))
           (let* ((expanded (mrr-search-plan-graph strategy-name (plan-node-step node)))
                  (children (car expanded))
                  (nested (cadr expanded))
                  (roots (filter (lambda (factor)
                                   (every (lambda (edge) (not (equal? (cadr edge) factor))) nested))
                                 children))
                  (terminals (filter (lambda (factor)
                                       (every (lambda (edge) (not (equal? (car edge) factor))) nested))
                                     children)))
             (when (null? children) (error "empty POO Search branch" id))
             (set! factors (append factors children))
             (set! edges (append edges nested
                                  (append-map (lambda (from)
                                                (map (lambda (to) (list from to)) roots)) previous)))
             (set! boundaries (cons (cons id terminals) boundaries))))
          ((eq? kind 'external)
           (let (factor (string-append "mrr.search.factor.v1:"
                                       (symbol->string strategy-name) ":"
                                       (symbol->string (plan-node-name node))))
             (set! factors (append factors (list factor)))
             (set! edges (append edges (map (lambda (from) (list from factor)) previous)))
             (set! boundaries (cons (cons id (list factor)) boundaries))))
          (else (error "unsupported POO Search plan node" kind)))))
     nodes)
    (list factors (delete-duplicates/hash edges))))

(def (mrr-search-same-set? left right)
  (and (= (length left) (length right))
       (= (length left) (length (delete-duplicates/hash left)))
       (= (length right) (length (delete-duplicates/hash right)))
       (every (lambda (item) (member item right)) left)))

(def (mrr-stable-search-name? name)
  (and (symbol? name)
       (let (text (symbol->string name))
         (and (> (string-length text) 0)
              (every (lambda (char)
                       (or (char<=? #\a char #\z)
                           (char<=? #\0 char #\9)
                           (char=? char #\-)))
                     (string->list text))))))

(def (mrr-search-project strategy-value generation-value)
  (unless (and (poo-flow-search-strategy? strategy-value)
               (string? generation-value)
               (> (string-length generation-value) 0))
    (error "MRR Search projection requires a POO strategy and generation"
           strategy-value generation-value))
  (let* ((name-value (.ref strategy-value 'name))
         (root-value (.ref strategy-value 'root))
         (stages (mrr-search-stages root-value))
         (names (map (lambda (stage) (.ref stage 'name)) stages)))
    (unless (and (mrr-stable-search-name? name-value)
                 (every mrr-stable-search-name? names)
                 (= (length names) (length (delete-duplicates/hash names))))
      (error "MRR Search factors require distinct stable names" name-value names))
    (let* ((flow (poo-flow-search-node-flow root-value))
           (compiled (flow->linear-plan flow))
           (expanded (mrr-search-plan-graph name-value flow))
           (factor-inputs (map (lambda (stage) (mrr-search-factor-input name-value stage)) stages))
           (tree-edges (mrr-search-edges name-value root-value)))
      (unless (and (equal? (.ref strategy-value 'dag-receipt)
                           (execution-plan->dag-receipt compiled))
                   (mrr-search-same-set? factor-inputs (car expanded))
                   (mrr-search-same-set? tree-edges (cadr expanded)))
        (error "POO DAG and Search projection correspondence mismatch" name-value)))
    (poo-core-role-object
     (slots ((kind 'mrr-search-projection)
             (strategy strategy-value)
             (generation-canonical-input generation-value)
             (factor-rows
              (map (lambda (stage)
                     (list (mrr-search-factor-input name-value stage)
                           (symbol->string (.ref stage 'search/stage-role))))
                   stages))
             (factor-edges (mrr-search-edges name-value root-value))
             (dag-receipt (.ref strategy-value 'dag-receipt))))
     (supers))))

(def (mrr-search-factor-observation
      projection-value observed-generation identity subject factor candidate-identity
      logical-position provenance-identity causal-parent-identities modality committed?)
  (unless (and (mrr-search-projection? projection-value)
               (poo-flow-search-stage? factor)
               (memq factor (mrr-search-stages
                             (.ref (.ref projection-value 'strategy) 'root)))
               (equal? observed-generation
                       (.ref projection-value 'generation-canonical-input)))
    (error "Search observation is outside its strategy or generation"
           identity observed-generation))
  (poo-flow-search-factor-observation
   identity subject factor candidate-identity logical-position
   provenance-identity causal-parent-identities modality committed?))

;;; Direct Scheme API: inference consumes the original POO projection.
(def (mrr-search-evaluate projection generation observed-stages)
  (unless (and (mrr-search-projection? projection)
               (equal? generation (.ref projection 'generation-canonical-input))
               (list? observed-stages))
    (error "invalid Search projection generation"))
  (let* ((strategy (.ref projection 'strategy))
         (stages (mrr-search-stages (.ref strategy 'root)))
         (names (map car (.ref projection 'factor-rows))))
    (unless (every (lambda (stage) (memq stage stages)) observed-stages)
      (error "foreign Search observation stage"))
    (def (coordinate name)
      (let loop ((rest names) (index 0))
        (if (equal? name (car rest)) index (loop (cdr rest) (+ index 1)))))
    (let* ((edges (map (lambda (edge) (map coordinate edge))
                       (.ref projection 'factor-edges)))
           (observed (map (lambda (stage)
                            (coordinate (mrr-search-factor-input (.ref strategy 'name) stage)))
                          observed-stages))
           (answer (mrr-finite-evaluate (mrr-finite-request (length names) edges observed))))
      (poo-core-role-object
       (slots ((kind 'mrr-search-result)
               (projection projection)
               (generation-canonical-input generation)
               (paths (map (lambda (row)
                             (list (list-ref names (car row))
                                   (list-ref names (cadr row)) (caddr row)))
                           (.ref answer 'paths)))
               (influences (map (lambda (row)
                                  (list (car row) (list-ref names (cadr row))
                                        (list-ref names (caddr row)) (cadddr row)))
                                (.ref answer 'influences)))))
       (supers)))))

;;; Packet-local aliases name actual POO objects by identity, not inherited slot
;;; text. Supers and runtime C4 outputs are read from those same objects.
(def (mrr-search-role-evidence strategy)
  (let ((objects '()) (rows '()) (next 0))
    (def (identify object)
      (let (found (assq object objects))
        (if found (cdr found)
          (begin
            (when (>= next 512) (error "Search role graph exceeds retained evidence bound"))
            (let (id (string-append "r" (number->string next)))
              (set! next (+ next 1))
              (set! objects (cons (cons object id) objects))
              (let (parents (map identify (object-supers object)))
                (set! rows (append rows (list (list id parents)))))
              id)))))
    (let* ((stages (mrr-search-stages (.ref strategy 'root)))
           (name (.ref strategy 'name))
           (roots (map (lambda (stage)
                         (list (mrr-search-factor-input name stage) (identify stage))) stages))
           (orders (map (lambda (stage)
                          (list (mrr-search-factor-input name stage)
                                (map identify (compute-precedence-list! stage)))) stages)))
      (list "mrr.poo.search.roles.v1" rows roots orders))))

;;; Closed inert transport on the existing host bridge, not a public POO DSL.
(def (mrr-search-build-strategy request)
  (def (name text)
    (unless (and (string? text) (<= (string-length text) 128)
                 (mrr-stable-search-name? (string->symbol text)))
      (error "invalid Search plan name"))
    (string->symbol text))
  (def (role text)
    (cond
     ((equal? text "acquisition") poo-flow-search-acquisition-role)
     ((equal? text "refinement") poo-flow-search-refinement-role)
     ((equal? text "reasoning") poo-flow-search-reasoning-role)
     ((equal? text "projection") poo-flow-search-projection-role)
     (else (error "invalid Search plan role"))))
  (def (node row depth)
    (unless (and (list? row) (>= (length row) 3) (<= depth 24))
      (error "invalid Search plan node"))
    (let ((kind (car row)) (id (name (cadr row))))
      (cond
       ((equal? kind "stage")
        (unless (and (= (length row) 5) (string? (list-ref row 3))
                     (string? (list-ref row 4))
                     (> (string-length (list-ref row 3)) 0)
                     (> (string-length (list-ref row 4)) 0))
          (error "invalid Search plan stage"))
        (poo-flow-search-stage id id '() (list-ref row 3) (list-ref row 4)
                               (role (list-ref row 2))))
       ((or (equal? kind "chain") (equal? kind "parallel"))
        (unless (and (= (length row) 3) (list? (caddr row)))
          (error "invalid Search plan children"))
        ((if (equal? kind "chain") poo-flow-search-chain poo-flow-search-parallel)
         id (map (lambda (child) (node child (+ depth 1))) (caddr row))))
       ((equal? kind "merge")
        (unless (and (= (length row) 6) (string? (list-ref row 5))
                     (> (string-length (list-ref row 5)) 0))
          (error "invalid Search plan merge"))
        (let* ((parallel (node (caddr row) (+ depth 1)))
               (stage (poo-flow-search-stage
                       (name (list-ref row 3)) (name (list-ref row 3)) '()
                       (poo-flow-search-node-output-domain parallel)
                       (list-ref row 5) (role (list-ref row 4)))))
          (poo-flow-search-merge id parallel stage)))
       (else (error "invalid Search plan constructor")))))
  (unless (and (list? request) (= (length request) 3))
    (error "invalid Search plan request"))
  (poo-flow-search-strategy (name (car request)) (node (caddr request) 0) '()))

(def (mrr-search-compile payload)
  (let* ((request (scheme-wire-read payload))
         (strategy (mrr-search-build-strategy request)))
    (mrr-search-compile-strategy strategy (cadr request))))

(def (mrr-search-compile-strategy strategy generation)
  (let* ((projection (mrr-search-project strategy generation))
         (answer (mrr-search-evaluate projection generation '()))
         (dag (call-with-output-string
               (lambda (port) (write (.ref projection 'dag-receipt) port)))))
    (list "mrr.poo.search.projection.v1" (symbol->string (.ref strategy 'name))
          (.ref projection 'generation-canonical-input)
          (.ref projection 'factor-rows) (.ref projection 'factor-edges) dag
          (.ref answer 'paths) (mrr-search-role-evidence strategy))))

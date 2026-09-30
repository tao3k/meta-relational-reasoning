;;; Backend-neutral POO Search composition framework.
;;;
;;; Consumers own their query languages, concrete operators, runtime execution,
;;; and result packets.  This module owns only typed POO composition.

(import (only-in :clan/poo/object .ref object?)
        :poo-flow/src/core/object-syntax
        (only-in :poo-flow/src/core/flow
                 external-flow
                 flow-fanout
                 flow-input-contract
                 flow-output-contract
                 flow-then)
        (only-in :poo-flow/src/core/plan flow->dag-receipt)
        (only-in :poo-flow/modules/temporal-causality/interface
                 poo-flow-causal-event
                 poo-flow-causal-event-graph
                 poo-flow-causal-trajectory-assess
                 poo-flow-causal-trajectory-contract
                 poo-flow-structural-impact-analyze
                 poo-flow-temporal-observation))

(export +search-framework-schema+
        search-framework-role
        search-acquisition-role
        search-refinement-role
        search-reasoning-role
        search-projection-role
        search-stage-prototype
        search-composition-prototype
        search-strategy-prototype
        make-search-stage
        make-search-chain
        make-search-parallel
        make-search-merge
        make-search-strategy
        make-search-factor-observation
        make-search-causal-event-graph
        make-search-causal-trajectory-contract
        assess-search-causal-trajectory
        search-structural-impact
        search-object-ref
        search-stage?
        search-composition?
        search-strategy?
        search-node-input-domain
        search-node-output-domain
        search-node-flow
        search-strategy-root
        search-strategy-generation-canonical-input
        search-strategy-factor-rows
        search-strategy-factor-edges
        search-strategy-policy
        search-strategy-dag-receipt)

(def +search-framework-schema+ 'mrr.poo-search-framework.v1)

(def search-framework-role
  (poo-core-role-object
   (slots ((search/framework? #t)
           (search/factor? #f)
           (control-owner 'consumer)
           (execution-owner 'consumer-runtime)))
   (supers)))

(def search-acquisition-role
  (poo-core-role-object
   (slots ((search/factor? #t)
           (search/stage-role 'acquisition)))
   (supers search-framework-role)))

(def search-refinement-role
  (poo-core-role-object
   (slots ((search/factor? #t)
           (search/stage-role 'refinement)))
   (supers search-framework-role)))

(def search-reasoning-role
  (poo-core-role-object
   (slots ((search/factor? #t)
           (search/stage-role 'reasoning)))
   (supers search-framework-role)))

(def search-projection-role
  (poo-core-role-object
   (slots ((search/factor? #t)
           (search/stage-role 'projection)))
   (supers search-framework-role)))

(def search-stage-prototype
  (poo-core-role-object
   (slots ((schema +search-framework-schema+)
           (kind 'search-stage)
           (name #f)
           (operation #f)
           (arguments '())
           (input-domain #f)
           (output-domain #f)
           (flow #f)
           (metadata '())))
   (supers search-framework-role)))

(def search-composition-prototype
  (poo-core-role-object
   (slots ((schema +search-framework-schema+)
           (kind 'search-composition)
           (name #f)
           (mode #f)
           (children '())
           (input-domain #f)
           (output-domain #f)
           (flow #f)
           (metadata '())))
   (supers search-framework-role)))

(def search-strategy-prototype
  (poo-core-role-object
   (slots ((schema +search-framework-schema+)
           (kind 'search-strategy)
           (name #f)
           (root #f)
           (generation-canonical-input #f)
           (factor-rows '())
           (factor-edges '())
           (policy '())
           (dag-receipt #f)
           (metadata '())))
   (supers search-framework-role)))

(def (search-object-ref object key)
  (.ref object key))

(def (search-kind? value expected)
  (and (object? value)
       (with-catch
        (lambda (_) #f)
        (lambda () (eq? (.ref value 'kind) expected)))))

(def (search-stage? value)
  (search-kind? value 'search-stage))

(def (search-composition? value)
  (search-kind? value 'search-composition))

(def (search-strategy? value)
  (search-kind? value 'search-strategy))

(def (search-node? value)
  (or (search-stage? value) (search-composition? value)))

(def (search-node-input-domain node)
  (unless (search-node? node)
    (error "expected a POO Search node" node))
  (.ref node 'input-domain))

(def (search-node-output-domain node)
  (unless (search-node? node)
    (error "expected a POO Search node" node))
  (.ref node 'output-domain))

(def (search-node-flow node)
  (unless (search-node? node)
    (error "expected a POO Search node" node))
  (.ref node 'flow))

(def (valid-stage-role? role)
  (and (object? role)
       (with-catch
        (lambda (_) #f)
        (lambda () (memq (.ref role 'search/stage-role)
                         '(acquisition refinement reasoning projection))))))

(def (make-search-stage name operation arguments input-domain output-domain role
                        . maybe-metadata)
  (unless (and (symbol? name)
               (symbol? operation)
               (list? arguments)
               input-domain
               output-domain
               (valid-stage-role? role)
               (or (null? maybe-metadata)
                   (and (null? (cdr maybe-metadata))
                        (list? (car maybe-metadata)))))
    (error "invalid POO Search stage" name operation input-domain output-domain))
  (let (flow (external-flow name operation arguments input-domain output-domain))
    (poo-core-role-object
     (slots ((name name)
             (operation operation)
             (arguments arguments)
             (input-domain input-domain)
             (output-domain output-domain)
             (flow flow)
             (metadata (if (null? maybe-metadata) '() (car maybe-metadata)))))
     (supers role search-stage-prototype))))

(def (make-search-composition name mode children flow metadata)
  (poo-core-role-object
   (slots ((name name)
           (mode mode)
           (children children)
           (input-domain (flow-input-contract flow))
           (output-domain (flow-output-contract flow))
           (flow flow)
           (metadata metadata)))
   (supers search-composition-prototype)))

(def (make-search-chain name children . maybe-metadata)
  (unless (and (symbol? name) (pair? children))
    (error "Search chain requires a name and at least one node" name children))
  (let loop ((remaining children) (combined #f) (previous #f))
    (if (null? remaining)
      (make-search-composition
       name 'sequential children combined
       (if (null? maybe-metadata) '() (car maybe-metadata)))
      (let (node (car remaining))
        (unless (search-node? node)
          (error "Search chain child is not a POO Search node" node))
        (when (and previous
                   (not (equal? (search-node-output-domain previous)
                                (search-node-input-domain node))))
          (error "Search chain domain mismatch"
                 (search-node-output-domain previous)
                 (search-node-input-domain node)))
        (loop (cdr remaining)
              (if combined
                (flow-then name combined (search-node-flow node))
                (search-node-flow node))
              node)))))

(def (make-search-parallel name children . maybe-metadata)
  (unless (and (symbol? name) (pair? children) (pair? (cdr children)))
    (error "Search parallel composition requires at least two nodes" name children))
  (let ((input-domain (search-node-input-domain (car children))))
    (let loop ((remaining children) (combined #f))
      (if (null? remaining)
        (make-search-composition
         name 'parallel children combined
         (if (null? maybe-metadata) '() (car maybe-metadata)))
        (let (node (car remaining))
          (unless (and (search-node? node)
                       (equal? input-domain (search-node-input-domain node)))
            (error "Search parallel branch domain mismatch" node input-domain))
          (loop (cdr remaining)
                (if combined
                  (flow-fanout name combined (search-node-flow node))
                  (search-node-flow node))))))))

(def (make-search-merge name parallel merge-stage . maybe-metadata)
  (unless (and (search-composition? parallel)
               (eq? (.ref parallel 'mode) 'parallel)
               (search-stage? merge-stage)
               (equal? (search-node-output-domain parallel)
                       (search-node-input-domain merge-stage)))
    (error "Search merge requires a compatible parallel composition and stage"
           parallel merge-stage))
  (make-search-composition
   name 'merge (list parallel merge-stage)
   (flow-then name (search-node-flow parallel) (search-node-flow merge-stage))
   (if (null? maybe-metadata) '() (car maybe-metadata))))

(def (stable-search-name? name)
  (and (symbol? name)
       (let (characters (string->list (symbol->string name)))
         (and (pair? characters)
              (let loop ((remaining characters))
                (or (null? remaining)
                    (let (code (char->integer (car remaining)))
                      (and (or (and (>= code 97) (<= code 122))
                               (and (>= code 48) (<= code 57))
                               (= code 45))
                           (loop (cdr remaining))))))))))

(def (search-node-stages node)
  (if (search-stage? node)
    (list node)
    (apply append (map search-node-stages (.ref node 'children)))))

(def (search-node-sources node)
  (if (search-stage? node)
    (list node)
    (let (children (.ref node 'children))
      (if (eq? (.ref node 'mode) 'parallel)
        (apply append (map search-node-sources children))
        (search-node-sources (car children))))))

(def (search-node-sinks node)
  (if (search-stage? node)
    (list node)
    (let (children (.ref node 'children))
      (if (eq? (.ref node 'mode) 'parallel)
        (apply append (map search-node-sinks children))
        (search-node-sinks (car (reverse children)))))))

(def (search-factor-canonical-input strategy-name stage)
  (string-append "mrr.search.factor.v1:"
                 (symbol->string strategy-name) ":"
                 (symbol->string (.ref stage 'name))))

(def (distinct-search-names? names)
  (or (null? names)
      (and (not (memq (car names) (cdr names)))
           (distinct-search-names? (cdr names)))))

(def (search-node-factor-edges strategy-name node)
  (if (search-stage? node)
    '()
    (let* ((children (.ref node 'children))
           (nested (apply append
                          (map (lambda (child)
                                 (search-node-factor-edges strategy-name child))
                               children))))
      (if (eq? (.ref node 'mode) 'parallel)
        nested
        (let loop ((remaining children) (edge-batches '()))
          (if (or (null? remaining) (null? (cdr remaining)))
            (append nested (apply append (reverse edge-batches)))
            (let* ((left-sinks (search-node-sinks (car remaining)))
                   (right-sources (search-node-sources (cadr remaining)))
                   (links
                    (apply append
                           (map (lambda (left)
                                  (map (lambda (right)
                                         (list (search-factor-canonical-input strategy-name left)
                                               (search-factor-canonical-input strategy-name right)))
                                       right-sources))
                                left-sinks))))
              (loop (cdr remaining) (cons links edge-batches)))))))))

(def (make-search-strategy name root generation-canonical-input policy
                           . maybe-metadata)
  (unless (and (stable-search-name? name)
               (search-node? root)
               (string? generation-canonical-input)
               (> (string-length generation-canonical-input) 0)
               (list? policy))
    (error "invalid POO Search strategy" name root generation-canonical-input))
  (let* ((stages (search-node-stages root))
         (stage-names (map (lambda (stage) (.ref stage 'name)) stages)))
    (unless (and (every stable-search-name? stage-names)
                 (distinct-search-names? stage-names))
      (error "Search strategy requires distinct stable factor names" stage-names))
    (poo-core-role-object
     (slots ((name name)
             (root root)
             (generation-canonical-input generation-canonical-input)
             (factor-rows
              (map (lambda (stage)
                     (list (search-factor-canonical-input name stage)
                           (symbol->string (.ref stage 'search/stage-role))))
                   stages))
             (factor-edges (search-node-factor-edges name root))
             (policy policy)
             (dag-receipt (flow->dag-receipt (search-node-flow root)))
             (metadata (if (null? maybe-metadata) '() (car maybe-metadata)))))
     (supers search-strategy-prototype))))

;;; Search observations reuse POO Flow's temporal-causality authority.  The
;;; factor name is the event kind; the candidate identity is the payload.
(def (make-search-factor-observation
      strategy observed-generation identity subject factor candidate-identity
      logical-position provenance-identity causal-parent-identities modality
      committed?)
  (unless (and (search-strategy? strategy)
               (search-stage? factor)
               (memq factor (search-node-stages (search-strategy-root strategy)))
               (equal? observed-generation
                       (search-strategy-generation-canonical-input strategy)))
    (error "Search observation is outside its strategy or generation"
           identity observed-generation))
  (let (observation
        (poo-flow-temporal-observation
         (string-append identity ":observation")
         'logical-version
         logical-position
         provenance-identity))
    (poo-flow-causal-event
     identity
     subject
     (.ref factor 'name)
     observation
     candidate-identity
     causal-parent-identities
     modality
     committed?)))

(def (make-search-causal-event-graph subject observations)
  (poo-flow-causal-event-graph subject observations))

(def (make-search-causal-trajectory-contract
      identity trigger-event-id intended-event-ids error-event-paths
      intended-impact-event-ids error-impact-event-ids)
  (poo-flow-causal-trajectory-contract
   identity trigger-event-id intended-event-ids error-event-paths
   intended-impact-event-ids error-impact-event-ids))

(def (assess-search-causal-trajectory contract event-graph)
  (poo-flow-causal-trajectory-assess contract event-graph))

(def (search-structural-impact
      graph changed-node-ids selected-relations direction inventory-complete?)
  (poo-flow-structural-impact-analyze
   graph changed-node-ids selected-relations direction inventory-complete?))

(def (search-strategy-root strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'root))

(def (search-strategy-generation-canonical-input strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'generation-canonical-input))

(def (search-strategy-factor-rows strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'factor-rows))

(def (search-strategy-factor-edges strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'factor-edges))

(def (search-strategy-policy strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'policy))

(def (search-strategy-dag-receipt strategy)
  (unless (search-strategy? strategy)
    (error "expected a POO Search strategy" strategy))
  (.ref strategy 'dag-receipt))

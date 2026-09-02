;;; Gerbil POO authority for ReasoningBundle declarations.

(import :clan/poo/object
        :poo-flow/src/core/object-syntax)
(export defmrr-reasoning-module
        mrr-reasoning-module-prototype
        mrr-driver-start
        mrr-driver-request
        mrr-driver-accept
        mrr-driver-result
        mrr-resource-receipt
        mrr-resource-ref)

(def mrr-reasoning-module-prototype
  (poo-core-role-object
   (slots ((kind 'mrr-reasoning-module-prototype)
           (schema "mrr.reasoning-bundle.v1")))
   (supers)))

(defsyntax (defmrr-reasoning-module stx)
  (syntax-case stx
      (module relation-schemas query-templates rule-packs inverse-goals
              transition-systems resource-language reasoning-loop
              lineage-policy projection-policy validation-profile)
    ((_ binding
        (module module-name)
        (relation-schemas
         (schema-name schema-cardinality
                      (field-name field-type) ...) ...)
        (query-templates
         (query-name query-relation (query-dependency ...)) ...)
        (rule-packs
         (pack-name rule-name head-relation (body-relation ...)) ...)
        (inverse-goals
         (inverse-name inverse-query) ...)
        (transition-systems
         (system-name (system-relation ...)) ...)
        (resource-language
         (resource-name resource-origin resource-authority) ...)
        (reasoning-loop
         (from-phase loop-resource receipt-status to-phase) ...)
        (lineage-policy lineage-mode)
        (projection-policy include-source? include-intermediate?)
        (validation-profile max-query-depth require-complete?))
     (identifier? #'binding)
     #'(def binding
         (poo-core-role-object
          (slots
           ((kind 'mrr-reasoning-module)
            (reasoning-module-id 'module-name)
            (relation-schemas
             (list
              (list 'schema-name 'schema-cardinality
                    (list (list 'field-name 'field-type) ...)) ...))
            (query-templates
             (list
              (list 'query-name 'query-relation
                    (list 'query-dependency ...)) ...))
            (rule-packs
             (list
              (list 'pack-name 'rule-name 'head-relation
                    (list 'body-relation ...)) ...))
            (inverse-goals
             (list (list 'inverse-name 'inverse-query) ...))
            (transition-systems
             (list
              (list 'system-name (list 'system-relation ...)) ...))
            (resource-language
             (list
              (list 'resource-name 'resource-origin
                    'resource-authority) ...))
            (reasoning-loop
             (list
              (list 'from-phase 'loop-resource 'receipt-status
                    'to-phase) ...))
            (lineage-policy 'lineage-mode)
            (projection-policy
             (list include-source? include-intermediate?))
            (validation-profile
             (list max-query-depth require-complete?))))
          (supers mrr-reasoning-module-prototype))))
    (_
     (raise-syntax-error
      #f
      "invalid MRR reasoning module declaration"
      stx))))

;;; The outer loop is deliberately small and deterministic.  It chooses which
;;; resource runs next, but it never interprets facts, runs rules, assigns
;;; identities, or admits a closure.  Those decisions remain Rust/Ascent-owned.

(def (mrr-driver-object object-kind fields)
  (poo-core-role-object
   (slots ((kind object-kind)
           (fields fields)))
   (supers)))

(def (mrr-driver-ref object key)
  (let ((entry (assq key (.ref object 'fields))))
    (and entry (cdr entry))))

(def mrr-resource-ref mrr-driver-ref)

(def (mrr-resource-receipt resource status cycle payload)
  (unless (and (symbol? resource) (symbol? status)
               (integer? cycle) (>= cycle 0))
    (error "invalid MRR resource receipt fields"
           resource status cycle))
  (mrr-driver-object
   'mrr-resource-receipt
   `((resource . ,resource)
     (status . ,status)
     (cycle . ,cycle)
     (payload . ,payload))))

(def (mrr-driver-state phase cycle max-cycles task visible-world
                       proposal result schedule resources)
  (mrr-driver-object
   'mrr-driver-state
   `((phase . ,phase)
     (cycle . ,cycle)
     (max-cycles . ,max-cycles)
     (task . ,task)
     (visible-world . ,visible-world)
     (proposal . ,proposal)
     (result . ,result)
     (schedule . ,schedule)
     (resources . ,resources))))

(def (mrr-driver-start reasoning-module task visible-world max-cycles)
  (unless (and (integer? max-cycles) (> max-cycles 0))
    (error "MRR driver requires a positive cycle budget" max-cycles))
  (unless (eq? (.ref reasoning-module 'kind) 'mrr-reasoning-module)
    (error "MRR driver requires a declared reasoning module" reasoning-module))
  (let ((schedule (.ref reasoning-module 'reasoning-loop))
        (resources (.ref reasoning-module 'resource-language)))
    (when (null? schedule)
      (error "MRR driver requires a non-empty reasoning loop" reasoning-module))
    (mrr-driver-state
     (car (car schedule)) 0 max-cycles task visible-world #f #f
     schedule resources)))

(def (mrr-driver-transition-row state phase resource status)
  (let loop ((rows (mrr-driver-ref state 'schedule)))
    (cond
     ((null? rows) #f)
     ((and (eq? phase (list-ref (car rows) 0))
           (or (not resource) (eq? resource (list-ref (car rows) 1)))
           (or (not status) (eq? status (list-ref (car rows) 2))))
      (car rows))
     (else (loop (cdr rows))))))

(def (mrr-driver-resource-authority state resource)
  (let ((entry (assq resource (mrr-driver-ref state 'resources))))
    (and entry (list-ref entry 2))))

(def (mrr-driver-request state)
  (unless (eq? (.ref state 'kind) 'mrr-driver-state)
    (error "invalid MRR driver state" state))
  (let ((phase (mrr-driver-ref state 'phase))
        (cycle (mrr-driver-ref state 'cycle)))
    (let ((row (mrr-driver-transition-row state phase #f #f)))
      (if row
       (mrr-driver-object
        'mrr-resource-request
        `((resource . ,(list-ref row 1))
          (cycle . ,cycle)
          (payload . (,(mrr-driver-ref state 'task)
                      ,(mrr-driver-ref state 'visible-world)
                      ,(mrr-driver-ref state 'proposal)))))
       #f))))

(def (mrr-driver-accept state receipt)
  (unless (eq? (.ref receipt 'kind) 'mrr-resource-receipt)
    (error "invalid MRR resource receipt" receipt))
  (let* ((request (mrr-driver-request state))
         (phase (mrr-driver-ref state 'phase))
         (cycle (mrr-driver-ref state 'cycle))
         (max-cycles (mrr-driver-ref state 'max-cycles))
         (resource (mrr-driver-ref receipt 'resource))
         (status (mrr-driver-ref receipt 'status))
         (payload (mrr-driver-ref receipt 'payload))
         (row (mrr-driver-transition-row state phase resource status)))
    (unless (and request
                 (eq? resource (mrr-driver-ref request 'resource))
                 (equal? cycle (mrr-driver-ref receipt 'cycle)))
      (error "MRR resource receipt does not match pending request" receipt))
    (unless row
      (error "resource receipt is not declared by the reasoning loop" receipt))
    (let* ((next-phase (list-ref row 3))
           (initial-phase
            (car (car (mrr-driver-ref state 'schedule))))
           (retry? (and (eq? next-phase initial-phase)
                        (not (eq? phase initial-phase))))
           (next-cycle (if retry? (+ cycle 1) cycle))
           (authoritative?
            (eq? (mrr-driver-resource-authority state resource)
                 'authoritative))
           (terminal?
            (not (mrr-driver-transition-row
                  state next-phase #f #f))))
      (when (and retry? (>= next-cycle max-cycles))
        (error "MRR driver exhausted its cycle budget" max-cycles))
      (mrr-driver-state
       next-phase next-cycle max-cycles
       (mrr-driver-ref state 'task)
       (mrr-driver-ref state 'visible-world)
       (if authoritative?
           (mrr-driver-ref state 'proposal)
           payload)
       (and authoritative? terminal? payload)
       (mrr-driver-ref state 'schedule)
       (mrr-driver-ref state 'resources)))))

(def (mrr-driver-result state)
  (unless (eq? (mrr-driver-ref state 'phase) 'complete)
    (error "MRR driver result requested before admission" state))
  (mrr-driver-ref state 'result))

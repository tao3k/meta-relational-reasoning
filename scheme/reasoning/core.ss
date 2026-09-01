;;; Gerbil POO authority for ReasoningBundle declarations.

(import :clan/poo/object
        :poo-flow/src/core/object-syntax)
(export defmrr-reasoning-module
        mrr-reasoning-module-prototype)

(def mrr-reasoning-module-prototype
  (poo-core-role-object
   (slots ((kind 'mrr-reasoning-module-prototype)
           (schema "mrr.reasoning-bundle.v1")))
   (supers)))

(defsyntax (defmrr-reasoning-module stx)
  (syntax-case stx
      (module relation-schemas query-templates rule-packs inverse-goals
              transition-systems lineage-policy projection-policy
              validation-profile)
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

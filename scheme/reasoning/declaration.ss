;;; Single-source bounded reasoning program used by POO and native AOT owners.

(defsyntax (with-mrr-reasoning-module stx)
  (syntax-case stx ()
    ((_ consumer binding)
     #'(consumer binding
         (module dependency-closure)
         (relation-schemas
          (edge many-to-many (from string) (to string))
          (reachable many-to-many (from string) (to string)))
         (query-templates
          (reachable-query reachable ()))
         (rule-packs
          (dependency-closure base reachable (edge))
          (dependency-closure transitive reachable (reachable edge)))
         (inverse-goals
          (why-not-reachable reachable-query))
         (transition-systems
          (closure-publication (reachable)))
         (resource-language
          (model-proposal external observational)
          (mrr-closure internal authoritative)
          (trajectory-sink external observational))
         (reasoning-loop
          (await-proposal model-proposal candidate await-closure)
          (await-closure mrr-closure admitted complete)
          (await-closure mrr-closure rejected await-proposal))
         (lineage-policy complete)
         (projection-policy #t #t)
         (validation-profile 64 #t)))))

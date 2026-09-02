;;; ISO GQL language-profile inventory owned by Gerbil and projected through AOT.
;;; Statuses are implementation evidence states, never ISO certification claims.

(export with-mrr-iso-gql-profile)

(defsyntax (with-mrr-iso-gql-profile stx)
  (syntax-case stx ()
    ((_ consumer binding)
     #'(consumer binding
         (schema mrr.iso-gql-profile.v1)
         (releases
          (iso-39075-2024 "ISO/IEC 39075:2024" published-edition
           published-edition-to-be-revised)
          (iso-39075-2024-cor-1 "ISO/IEC 39075:2024/Cor 1:2026"
           published-technical-corrigendum pending-licensed-clause))
         (modules
          (language-foundation iso-standard-module)
          (graph-model iso-standard-module)
          (path-patterns iso-standard-module)
          (query-core iso-standard-module)
          (query-advanced iso-standard-module)
          (data-management iso-standard-module))
         (profiles
          (gql-query-language-frontend-v1 iso-39075-2024
           independent-gql-compatible-query-language-frontend)
          (gql-iso-audit-v1 iso-39075-2024 audit-inventory-only)
          (gql-iso-language-frontend-v1 iso-39075-2024
           independent-full-iso-language-frontend-target))
         (profile-supplements
          (gql-iso-language-frontend-v1 iso-39075-2024-cor-1))
         (profile-modules
          (gql-query-language-frontend-v1 included language-foundation)
          (gql-query-language-frontend-v1 included graph-model)
          (gql-query-language-frontend-v1 included query-core)
          (gql-query-language-frontend-v1 deferred path-patterns)
          (gql-query-language-frontend-v1 deferred query-advanced)
          (gql-query-language-frontend-v1 deferred data-management)
          (gql-iso-audit-v1 included language-foundation)
          (gql-iso-audit-v1 included graph-model)
          (gql-iso-audit-v1 included path-patterns)
          (gql-iso-audit-v1 included query-core)
          (gql-iso-audit-v1 included query-advanced)
          (gql-iso-audit-v1 included data-management)
          (gql-iso-language-frontend-v1 included language-foundation)
          (gql-iso-language-frontend-v1 included graph-model)
          (gql-iso-language-frontend-v1 included path-patterns)
          (gql-iso-language-frontend-v1 included query-core)
          (gql-iso-language-frontend-v1 included query-advanced)
          (gql-iso-language-frontend-v1 included data-management))
         (features
          ;; id priority module clause syntax ast sema ir catalog evidence-owner
          (gql-lexical-identifiers 10 language-foundation pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/identifier_contract.rs")
          (gql-lexical-literals 20 language-foundation pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/literal_contract.rs")
          (gql-trivia-and-comments 30 language-foundation pending-licensed-clause
           partial not-applicable not-applicable not-applicable not-applicable
           "crates/gql/tests/unit/trivia_contract.rs")
          (gql-values-and-types 40 language-foundation pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/values_contract.rs")
          (gql-property-graph-model 50 graph-model pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/graph_model_contract.rs")
          (gql-catalog-and-schema-model 60 graph-model pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/catalog_schema_contract.rs")
          (gql-node-patterns 70 graph-model pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/node_pattern_contract.rs")
          (gql-edge-patterns 80 graph-model pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/edge_pattern_contract.rs")
          (gql-path-patterns 90 path-patterns pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/path_pattern_contract.rs")
          (gql-quantified-paths 100 path-patterns pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/quantified_path_contract.rs")
          (gql-expression-language 110 query-core pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/expression_language_contract.rs")
          (gql-property-and-label-expressions 120 query-core pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/property_label_expression_contract.rs")
          (gql-match 130 query-core pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/match_contract.rs")
          (gql-optional-match 140 query-core pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/optional_match_contract.rs")
          (gql-where 150 query-core pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/where_contract.rs")
          (gql-let 160 query-core pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/query_pipeline_contract.rs")
          (gql-return 170 query-core pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/query_pipeline_contract.rs")
          (gql-query-composition 180 query-core pending-licensed-clause
           partial partial partial partial not-applicable
           "crates/gql/tests/unit/query_pipeline_contract.rs")
          (gql-grouping-aggregation-ordering 190 query-advanced
           pending-licensed-clause partial partial partial partial not-applicable
           "crates/gql/tests/unit/query_pipeline_contract.rs")
          (gql-graph-modification 200 data-management pending-licensed-clause
           partial partial partial partial
           not-applicable "crates/gql/tests/unit/data_management_contract.rs")
          (gql-catalog-management 210 data-management pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/data_management_contract.rs")
          (gql-procedures-and-call 220 data-management pending-licensed-clause
           partial partial partial partial partial
           "crates/gql/tests/unit/data_management_contract.rs")
          (gql-session-and-control 230 data-management pending-licensed-clause
           partial partial partial partial
           not-applicable "crates/gql/tests/unit/data_management_contract.rs"))
         (feature-dependencies
          (gql-values-and-types gql-lexical-literals)
          (gql-property-graph-model gql-lexical-identifiers)
          (gql-property-graph-model gql-values-and-types)
          (gql-catalog-and-schema-model gql-property-graph-model)
          (gql-node-patterns gql-lexical-identifiers)
          (gql-node-patterns gql-property-graph-model)
          (gql-edge-patterns gql-node-patterns)
          (gql-path-patterns gql-node-patterns)
          (gql-path-patterns gql-edge-patterns)
          (gql-quantified-paths gql-path-patterns)
          (gql-expression-language gql-values-and-types)
          (gql-property-and-label-expressions gql-expression-language)
          (gql-property-and-label-expressions gql-property-graph-model)
          (gql-match gql-node-patterns)
          (gql-match gql-edge-patterns)
          (gql-optional-match gql-match)
          (gql-where gql-match)
          (gql-where gql-expression-language)
          (gql-let gql-lexical-identifiers)
          (gql-let gql-expression-language)
          (gql-return gql-expression-language)
          (gql-query-composition gql-return)
          (gql-grouping-aggregation-ordering gql-return)
          (gql-grouping-aggregation-ordering gql-expression-language)
          (gql-graph-modification gql-match)
          (gql-graph-modification gql-values-and-types)
          (gql-catalog-management gql-catalog-and-schema-model)
          (gql-procedures-and-call gql-catalog-and-schema-model)
          (gql-procedures-and-call gql-expression-language))))))

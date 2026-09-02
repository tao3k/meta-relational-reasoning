(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define meta-relational-reasoning/scheme/grammar/native::timestamp
    1788307577)
  (begin
    (define meta-relational-reasoning/scheme/grammar/native#mrr-native-grammar
      '((syntax-kinds
         (SourceFile node (query))
         (Query node (clause))
         (MatchClause node (mode patterns))
         (WhereClause node (expression))
         (LetClause node (binding))
         (LetBinding node (name value))
         (ReturnClause node (projection))
         (GraphPattern node (element))
         (GraphPatternList node (pattern))
         (NodePattern node (binding labels properties predicate))
         (PropertyMap node (entry))
         (PropertyEntry node (key value))
         (EdgePattern
          node
          (direction binding labels properties predicate quantifier))
         (LabelList node (label))
         (Expression node (token))
         (NameExpression node (name))
         (LiteralExpression node (literal))
         (CharacterStringLiteralExpression node (value form no-escape))
         (UnaryExpression node (operator operand))
         (BinaryExpression node (left operator right))
         (ParenthesizedExpression node (expression))
         (Keyword token (text))
         (Identifier token (text))
         (Number token (text))
         (String token (text))
         (ByteString token (text))
         (Whitespace token (text))
         (Punctuation token (text))
         (Comment token (text))
         (Unknown token (text))
         (PropertyAccessExpression node (base property))
         (FunctionCallExpression node (name argument))
         (PathPattern node (binding pattern))
         (PathMode node (kind))
         (PathQuantifier node (minimum maximum))
         (OptionalMatchClause node (match))
         (ListExpression node (element))
         (ByteStringLiteralExpression node (value))
         (TemporalLiteralExpression node (qualifier value))
         (DurationLiteralExpression node (value))
         (RecordExpression node (entry))
         (RecordEntry node (name value))
         (SubscriptExpression node (base index))
         (ProjectionAlias node (expression alias))
         (UnionClause node (query))
         (LimitClause node (limit))
         (OrderByClause node (key direction))
         (OffsetClause node (offset))
         (GroupByClause node (key))
         (CaseExpression node (operand branch else-result))
         (CaseWhenClause node (condition result))
         (CaseElseClause node (result))
         (CreateSchemaStatement node (name))
         (DropSchemaStatement node (name))
         (CreateGraphStatement node (name graph-type))
         (DropGraphStatement node (name))
         (CreateGraphTypeStatement node (name source policy))
         (DropGraphTypeStatement node (name policy))
         (CatalogObjectName node (part))
         (CatalogConflictClause node (kind))
         (GraphTypeSource node (kind target))
         (NestedGraphTypeSpecification node (element))
         (NodeTypeSpecification node (name alias key-labels labels properties))
         (EdgeTypeSpecification
          node
          (kind name endpoints direction key-labels labels properties))
         (EdgeKind node (kind))
         (EndpointPair node (endpoints direction))
         (NodeTypeReference node (alias key-labels labels properties))
         (EdgeDirection node (kind))
         (KeyLabelSet node (labels))
         (LabelSetPhrase node (labels))
         (PropertyTypeList node (property))
         (PropertyType node (name marker value-type))
         (PropertyValueType node (form item bound field nullability))
         (ValueTypeAtom node (kind parameter item field))
         (ReferenceValueType node (kind openness property specification field))
         (TypeParameterList node (value))
         (FieldTypeList node (field))
         (FieldType node (name marker value-type))
         (NotNullConstraint node (kind))
         (InsertStatement node (pattern))
         (SetStatement node (item))
         (SetItem node (target value))
         (RemoveStatement node (item))
         (RemoveItem node (target))
         (DeleteStatement node (item detach))
         (DeleteItem node (target))
         (CallStatement node (call))
         (ProcedureName node (part))
         (StartTransactionStatement node (access-mode))
         (CommitStatement node (action))
         (RollbackStatement node (action))
         (SessionSetStatement node (setting))
         (SessionResetStatement node (setting))
         (SessionCloseStatement node (action))
         (InlineWhereClause node (expression))
         (LabelPredicateExpression node (operand label))
         (LabelNameExpression node (name))
         (LabelWildcardExpression node (wildcard))
         (LabelNotExpression node (operand))
         (LabelAndExpression node (left right))
         (LabelOrExpression node (left right)))
        (keywords
         (Match "MATCH")
         (Optional "OPTIONAL")
         (Where "WHERE")
         (Let "LET")
         (Return "RETURN")
         (Or "OR")
         (Xor "XOR")
         (And "AND")
         (Not "NOT")
         (Call "CALL")
         (Create "CREATE")
         (Drop "DROP")
         (Insert "INSERT")
         (Delete "DELETE")
         (Set "SET")
         (Remove "REMOVE")
         (True "TRUE")
         (False "FALSE")
         (Null "NULL")
         (In "IN")
         (As "AS")
         (Union "UNION")
         (Limit "LIMIT")
         (Order "ORDER")
         (By "BY")
         (Group "GROUP")
         (Asc "ASC")
         (Desc "DESC")
         (Offset "OFFSET")
         (Case "CASE")
         (When "WHEN")
         (Then "THEN")
         (Else "ELSE")
         (End "END")
         (Schema "SCHEMA")
         (Session "SESSION")
         (Reset "RESET")
         (Close "CLOSE")
         (Property "PROPERTY")
         (Graph "GRAPH")
         (Any "ANY")
         (Typed "TYPED")
         (If "IF")
         (Exists "EXISTS")
         (Replace "REPLACE")
         (Type "TYPE")
         (Copy "COPY")
         (Of "OF")
         (Like "LIKE")
         (Detach "DETACH")
         (Nodetach "NODETACH")
         (Start "START")
         (Transaction "TRANSACTION")
         (Read "READ")
         (Only "ONLY")
         (Write "WRITE")
         (Commit "COMMIT")
         (Rollback "ROLLBACK")
         (Walk "WALK")
         (Trail "TRAIL")
         (Acyclic "ACYCLIC")
         (Simple "SIMPLE")
         (Is "IS")
         (Labeled "LABELED")
         (Date "DATE")
         (Time "TIME")
         (Timestamp "TIMESTAMP")
         (Datetime "DATETIME")
         (Duration "DURATION")
         (Record "RECORD"))
        (non-reserved-words
         (ACYCLIC)
         (BINDING)
         (BINDINGS)
         (CONNECTING)
         (DESTINATION)
         (DIFFERENT)
         (DIRECTED)
         (EDGE)
         (EDGES)
         (ELEMENT)
         (ELEMENTS)
         (FIRST)
         (GRAPH)
         (GROUPS)
         (KEEP)
         (LABEL)
         (LABELED)
         (LABELS)
         (LAST)
         (NFC)
         (NFD)
         (NFKC)
         (NFKD)
         (NO)
         (NODE)
         (NORMALIZED)
         (ONLY)
         (ORDINALITY)
         (PROPERTY)
         (READ)
         (RELATIONSHIP)
         (RELATIONSHIPS)
         (REPEATABLE)
         (SHORTEST)
         (SIMPLE)
         (SOURCE)
         (TABLE)
         (TO)
         (TRAIL)
         (TRANSACTION)
         (TYPE)
         (UNDIRECTED)
         (VERTEX)
         (WALK)
         (WITHOUT)
         (WRITE)
         (ZONE))
        (numeric-literals
         (exact-scientific scientific M exact)
         (exact-common common M exact)
         (exact-common-unsuffixed common none exact)
         (exact-integer integer M exact)
         (unsigned-integer integer none integer)
         (approximate-scientific scientific FD approximate)
         (approximate-scientific-unsuffixed scientific none approximate)
         (approximate-common common FD approximate)
         (approximate-integer integer FD approximate))
        (character-string-literals
         (single-quoted quote escaped-or-doubled character-string)
         (double-quoted double-quote escaped-or-doubled character-string)
         (no-escape commercial-at preserve-representations raw)
         (escaped-reverse-solidus reverse-solidus decode scalar)
         (escaped-quote quote decode scalar)
         (escaped-double-quote double-quote decode scalar)
         (escaped-grave-accent grave-accent decode scalar)
         (escaped-tab t decode control)
         (escaped-backspace b decode control)
         (escaped-new-line n decode control)
         (escaped-carriage-return r decode control)
         (escaped-form-feed f decode control)
         (escaped-unicode4 u decode four-hex-digits)
         (escaped-unicode6 U decode six-hex-digits))
        (prefix-operators
         (keyword Not 25 right)
         (punctuation "+" 60 right)
         (punctuation "-" 60 right))
        (binary-operators
         (keyword Or 10 left)
         (keyword Xor 15 left)
         (keyword And 20 left)
         (keyword In 30 left)
         (punctuation "=" 30 left)
         (punctuation "<>" 30 left)
         (punctuation "<" 30 left)
         (punctuation "<=" 30 left)
         (punctuation ">" 30 left)
         (punctuation ">=" 30 left)
         (punctuation "||" 35 left)
         (punctuation "+" 40 left)
         (punctuation "-" 40 left)
         (punctuation "*" 50 left)
         (punctuation "/" 50 left)
         (punctuation "%" 50 left))
        (parser-entrypoints
         (Match MatchClause marks-match)
         (Optional OptionalMatchClause marks-match)
         (Return ReturnClause marks-return)
         (Where WhereClause none)
         (Let LetClause none)
         (Union UnionClause none)
         (Limit LimitClause none)
         (Order OrderByClause none)
         (Offset OffsetClause none)
         (Group GroupByClause none)
         (Call CallStatement none)
         (Create CreateSchemaStatement none)
         (Drop DropSchemaStatement none)
         (Insert InsertStatement none)
         (Delete DeleteStatement none)
         (Set SetStatement none)
         (Remove RemoveStatement none)
         (Detach DeleteStatement none)
         (Nodetach DeleteStatement none)
         (Start StartTransactionStatement none)
         (Commit CommitStatement none)
         (Rollback RollbackStatement none)
         (Session SessionSetStatement none))
        (recoveries
         (block-comment
          "GQL-SYNTAX-UNTERMINATED-BLOCK-COMMENT"
          preserve-source)
         (numeric-literal "GQL-SYNTAX-INVALID-NUMERIC-LITERAL" preserve-source)
         (integer-literal-range
          "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE"
          preserve-source)
         (edge-label-separator
          "GQL-PARSE-EDGE-LABEL-SEPARATOR"
          preserve-source)
         (create-schema "GQL-PARSE-CREATE-SCHEMA-SYNTAX" preserve-source)
         (drop-schema "GQL-PARSE-DROP-SCHEMA-SYNTAX" preserve-source)
         (create-graph "GQL-PARSE-CREATE-GRAPH-SYNTAX" preserve-source)
         (drop-graph "GQL-PARSE-DROP-GRAPH-SYNTAX" preserve-source)
         (create-graph-type
          "GQL-PARSE-CREATE-GRAPH-TYPE-SYNTAX"
          preserve-source)
         (nested-graph-type
          "GQL-PARSE-NESTED-GRAPH-TYPE-SYNTAX"
          preserve-source)
         (drop-graph-type "GQL-PARSE-DROP-GRAPH-TYPE-SYNTAX" preserve-source)
         (insert-statement "GQL-PARSE-INSERT-SYNTAX" preserve-source)
         (set-statement "GQL-PARSE-SET-SYNTAX" preserve-source)
         (remove-statement "GQL-PARSE-REMOVE-SYNTAX" preserve-source)
         (delete-statement "GQL-PARSE-DELETE-SYNTAX" preserve-source)
         (call-statement "GQL-PARSE-CALL-SYNTAX" preserve-source)
         (transaction-command "GQL-PARSE-TRANSACTION-SYNTAX" preserve-source)
         (session-command "GQL-PARSE-SESSION-COMMAND-SYNTAX" preserve-source)
         (inline-node-where "GQL-PARSE-INLINE-WHERE-SYNTAX" preserve-source)
         (inline-edge-where "GQL-PARSE-INLINE-WHERE-SYNTAX" preserve-source)
         (path-mode "GQL-PARSE-PATH-MODE-SYNTAX" preserve-source)
         (path-quantifier "GQL-PARSE-PATH-QUANTIFIER" preserve-source)
         (string-literal "GQL-SYNTAX-UNTERMINATED-STRING" preserve-source)
         (character-string-literal
          "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL"
          preserve-source)
         (byte-string-literal "GQL-SYNTAX-INVALID-BYTE-STRING" preserve-source)
         (temporal-literal
          "GQL-SYNTAX-INVALID-TEMPORAL-LITERAL"
          preserve-source)
         (duration-literal
          "GQL-SYNTAX-INVALID-DURATION-LITERAL"
          preserve-source)
         (list-literal "GQL-PARSE-LIST-SYNTAX" preserve-source)
         (record-literal "GQL-PARSE-RECORD-SYNTAX" preserve-source)
         (delimited-identifier
          "GQL-SYNTAX-UNTERMINATED-DELIMITED-IDENTIFIER"
          preserve-source)
         (identifier-escape
          "GQL-SYNTAX-INVALID-IDENTIFIER-ESCAPE"
          preserve-source)
         (binding-variable "GQL-PARSE-BINDING-VARIABLE-SYNTAX" preserve-source)
         (unsupported-statement
          "GQL-PARSE-UNSUPPORTED-STATEMENT"
          preserve-source)
         (unsupported-keyword-expression
          "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION"
          preserve-source)
         (non-iso-operator "GQL-PARSE-NON-ISO-OPERATOR" preserve-source)
         (label-expression "GQL-PARSE-LABEL-EXPRESSION" preserve-source)
         (match-pattern-list "GQL-PARSE-MATCH-PATTERN-LIST" preserve-source)
         (optional-match "GQL-PARSE-OPTIONAL-MATCH-SYNTAX" preserve-source)
         (where-clause "GQL-PARSE-WHERE-SYNTAX" preserve-source)
         (union-clause "GQL-PARSE-UNION-SYNTAX" preserve-source)
         (expression-syntax "GQL-PARSE-EXPRESSION-SYNTAX" preserve-source))))
    (define meta-relational-reasoning/scheme/grammar/native#mrr-native-profile
      '((schema (mrr.iso-gql-profile.v1))
        (releases
         (iso-39075-2024
          "ISO/IEC 39075:2024"
          published-edition
          published-edition-to-be-revised)
         (iso-39075-2024-cor-1
          "ISO/IEC 39075:2024/Cor 1:2026"
          published-technical-corrigendum
          pending-licensed-clause))
        (modules (language-foundation iso-standard-module)
                 (graph-model iso-standard-module)
                 (path-patterns iso-standard-module)
                 (query-core iso-standard-module)
                 (query-advanced iso-standard-module)
                 (data-management iso-standard-module))
        (profiles
         (gql-query-language-frontend-v1
          iso-39075-2024
          independent-gql-compatible-query-language-frontend)
         (gql-iso-audit-v1 iso-39075-2024 audit-inventory-only)
         (gql-iso-language-frontend-v1
          iso-39075-2024
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
         (gql-lexical-identifiers
          10
          language-foundation
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/identifier_contract.rs")
         (gql-lexical-literals
          20
          language-foundation
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/literal_contract.rs")
         (gql-trivia-and-comments
          30
          language-foundation
          pending-licensed-clause
          partial
          not-applicable
          not-applicable
          not-applicable
          not-applicable
          "crates/gql/tests/unit/trivia_contract.rs")
         (gql-values-and-types
          40
          language-foundation
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/values_contract.rs")
         (gql-property-graph-model
          50
          graph-model
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/graph_model_contract.rs")
         (gql-catalog-and-schema-model
          60
          graph-model
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/catalog_schema_contract.rs")
         (gql-node-patterns
          70
          graph-model
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/node_pattern_contract.rs")
         (gql-edge-patterns
          80
          graph-model
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/edge_pattern_contract.rs")
         (gql-path-patterns
          90
          path-patterns
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/path_pattern_contract.rs")
         (gql-quantified-paths
          100
          path-patterns
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/quantified_path_contract.rs")
         (gql-expression-language
          110
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/expression_language_contract.rs")
         (gql-property-and-label-expressions
          120
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/property_label_expression_contract.rs")
         (gql-match
          130
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/match_contract.rs")
         (gql-optional-match
          140
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/optional_match_contract.rs")
         (gql-where
          150
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/where_contract.rs")
         (gql-let 160
                  query-core
                  pending-licensed-clause
                  partial
                  partial
                  partial
                  partial
                  not-applicable
                  "crates/gql/tests/unit/query_pipeline_contract.rs")
         (gql-return
          170
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/query_pipeline_contract.rs")
         (gql-query-composition
          180
          query-core
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/query_pipeline_contract.rs")
         (gql-grouping-aggregation-ordering
          190
          query-advanced
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/query_pipeline_contract.rs")
         (gql-graph-modification
          200
          data-management
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/data_management_contract.rs")
         (gql-catalog-management
          210
          data-management
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/data_management_contract.rs")
         (gql-procedures-and-call
          220
          data-management
          pending-licensed-clause
          partial
          partial
          partial
          partial
          partial
          "crates/gql/tests/unit/data_management_contract.rs")
         (gql-session-and-control
          230
          data-management
          pending-licensed-clause
          partial
          partial
          partial
          partial
          not-applicable
          "crates/gql/tests/unit/data_management_contract.rs"))
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
         (gql-procedures-and-call gql-expression-language))))
    (define meta-relational-reasoning/scheme/grammar/native#mrr-native-reasoning-module
      '((relation-schemas
         (edge many-to-many ((from string) (to string)))
         (reachable many-to-many ((from string) (to string))))
        (query-templates (reachable-query reachable ()))
        (rules (dependency-closure base reachable (edge))
               (dependency-closure transitive reachable (reachable edge)))
        (inverse-goals (why-not-reachable reachable-query ()))
        (transition-systems (closure-publication (reachable)))
        (resource-language
         (model-proposal external observational ())
         (mrr-closure internal authoritative ())
         (trajectory-sink external observational ()))
        (reasoning-loop
         (await-proposal model-proposal candidate await-closure ())
         (await-closure mrr-closure admitted complete ())
         (await-closure mrr-closure rejected await-proposal ()))
        (lineage-policy (complete ()))
        (projection-policy (#t #t ()))
        (validation-profile (64 #t ()))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-table
      (lambda (_%key8520%_)
        (cdr (assq _%key8520%_
                   meta-relational-reasoning/scheme/grammar/native#mrr-native-grammar))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-row
      (lambda (_%table8517%_ _%index8518%_)
        (if (>= _%index8518%_ '0)
            (if (< _%index8518%_ (length _%table8517%_))
                (list-ref _%table8517%_ _%index8518%_)
                '#f)
            '#f)))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text
      (lambda (_%value8509%_)
        (if (symbol? _%value8509%_)
            (let () (declare (not safe)) (##symbol->string _%value8509%_))
            (if (string? _%value8509%_)
                _%value8509%_
                (if (number? _%value8509%_)
                    (let ()
                      (declare (not safe))
                      (##number->string _%value8509%_))
                    (if (eq? _%value8509%_ '#t)
                        '"true"
                        (if (eq? _%value8509%_ '#f) '"false" '#f)))))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text-length
      (lambda (_%value8505%_)
        (let ((_%text8507%_
               (meta-relational-reasoning/scheme/grammar/native#grammar-text
                _%value8505%_)))
          (if _%text8507%_ (string-length _%text8507%_) '-1))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text-char
      (lambda (_%value8500%_ _%index8501%_)
        (let ((_%text8503%_
               (meta-relational-reasoning/scheme/grammar/native#grammar-text
                _%value8500%_)))
          (if (and _%text8503%_
                   (>= _%index8501%_ '0)
                   (< _%index8501%_ (string-length _%text8503%_)))
              (let ((__tmp11918 (string-ref _%text8503%_ _%index8501%_)))
                (declare (not safe))
                (##char->integer __tmp11918))
              '-1))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-rows
      (lambda (_%table8484%_)
        (let ((_%$e8486%_ _%table8484%_))
          (let ((_%default84888492%_ (lambda () '#f))
                (_%table84898494%_
                 '#(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16)))
            (if (fixnum? _%$e8486%_)
                (if (and (let () (declare (not safe)) (##fx>= _%$e8486%_ '0))
                         (let () (declare (not safe)) (##fx< _%$e8486%_ '17)))
                    (let ((_%x8497%_
                           (let ()
                             (declare (not safe))
                             (##vector-ref _%table84898494%_ _%$e8486%_))))
                      (if (let () (declare (not safe)) (##fx< _%x8497%_ '8))
                          (if (let ()
                                (declare (not safe))
                                (##fx< _%x8497%_ '4))
                              (if (let ()
                                    (declare (not safe))
                                    (##fx< _%x8497%_ '2))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '0))
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'syntax-kinds)
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'keywords))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '2))
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'prefix-operators)
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'binary-operators)))
                              (if (let ()
                                    (declare (not safe))
                                    (##fx< _%x8497%_ '6))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '4))
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'parser-entrypoints)
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'recoveries))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '6))
                                      (cdr (assq 'releases
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile))
                                      (cdr (assq 'modules
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile)))))
                          (if (let ()
                                (declare (not safe))
                                (##fx< _%x8497%_ '12))
                              (if (let ()
                                    (declare (not safe))
                                    (##fx< _%x8497%_ '10))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '8))
                                      (cdr (assq 'profiles
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile))
                                      (cdr (assq 'profile-modules
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile)))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '10))
                                      (cdr (assq 'features
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile))
                                      (cdr (assq 'feature-dependencies
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile))))
                              (if (let ()
                                    (declare (not safe))
                                    (##fx< _%x8497%_ '14))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '12))
                                      (cdr (assq 'schema
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile))
                                      (cdr (assq 'profile-supplements
                                                 meta-relational-reasoning/scheme/grammar/native#mrr-native-profile)))
                                  (if (let ()
                                        (declare (not safe))
                                        (##fx= _%x8497%_ '14))
                                      (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                       'non-reserved-words)
                                      (if (let ()
                                            (declare (not safe))
                                            (##fx= _%x8497%_ '15))
                                          (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                           'numeric-literals)
                                          (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                           'character-string-literals)))))))
                    (_%default84888492%_))
                (_%default84888492%_))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-rows
      (lambda (_%table8466%_)
        (let ((_%key8482%_
               (let ((_%$e8468%_ _%table8466%_))
                 (let ((_%default84708474%_ (lambda () '#f))
                       (_%table84718476%_ '#(0 1 2 3 4 5 6 7 8 9)))
                   (if (fixnum? _%$e8468%_)
                       (if (and (let ()
                                  (declare (not safe))
                                  (##fx>= _%$e8468%_ '0))
                                (let ()
                                  (declare (not safe))
                                  (##fx< _%$e8468%_ '10)))
                           (let ((_%x8479%_
                                  (let ()
                                    (declare (not safe))
                                    (##vector-ref
                                     _%table84718476%_
                                     _%$e8468%_))))
                             (if (let ()
                                   (declare (not safe))
                                   (##fx< _%x8479%_ '5))
                                 (if (let ()
                                       (declare (not safe))
                                       (##fx< _%x8479%_ '2))
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x8479%_ '0))
                                         'relation-schemas
                                         'query-templates)
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x8479%_ '2))
                                         'rules
                                         (if (let ()
                                               (declare (not safe))
                                               (##fx= _%x8479%_ '3))
                                             'inverse-goals
                                             'transition-systems)))
                                 (if (let ()
                                       (declare (not safe))
                                       (##fx< _%x8479%_ '7))
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x8479%_ '5))
                                         'lineage-policy
                                         'projection-policy)
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x8479%_ '7))
                                         'validation-profile
                                         (if (let ()
                                               (declare (not safe))
                                               (##fx= _%x8479%_ '8))
                                             'resource-language
                                             'reasoning-loop)))))
                           (_%default84708474%_))
                       (_%default84708474%_))))))
          (if _%key8482%_
              (cdr (assq _%key8482%_
                         meta-relational-reasoning/scheme/grammar/native#mrr-native-reasoning-module))
              '#f))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-nested
      (lambda (_%entry8464%_)
        (if _%entry8464%_
            (list-ref _%entry8464%_ (- (length _%entry8464%_) '1))
            '#f)))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
      (lambda (_%entry8452%_ _%index8453%_ _%column8454%_)
        (let* ((_%nested8456%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-nested
                 _%entry8452%_))
               (_%value8458%_
                (if _%nested8456%_
                    (if (>= _%index8453%_ '0)
                        (if (< _%index8453%_ (length _%nested8456%_))
                            (list-ref _%nested8456%_ _%index8453%_)
                            '#f)
                        '#f)
                    '#f)))
          (if (and (pair? _%value8458%_)
                   (>= _%column8454%_ '0)
                   (< _%column8454%_
                      (let () (declare (not safe)) (##length _%value8458%_))))
              (list-ref _%value8458%_ _%column8454%_)
              (if (and _%value8458%_ (= _%column8454%_ '0))
                  _%value8458%_
                  '#f)))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-phase
      (lambda (_%code8447%_)
        (let ((_%$e8449%_ _%code8447%_))
          (if (eq? '0 _%$e8449%_)
              'await-proposal
              (if (eq? '1 _%$e8449%_)
                  'await-closure
                  (if (eq? '2 _%$e8449%_) 'complete '#f))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-resource
      (lambda (_%code8442%_)
        (let ((_%$e8444%_ _%code8442%_))
          (if (eq? '0 _%$e8444%_)
              'model-proposal
              (if (eq? '1 _%$e8444%_) 'mrr-closure '#f)))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-status
      (lambda (_%code8437%_)
        (let ((_%$e8439%_ _%code8437%_))
          (if (eq? '0 _%$e8439%_)
              'candidate
              (if (eq? '1 _%$e8439%_)
                  'admitted
                  (if (eq? '2 _%$e8439%_) 'rejected '#f))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-phase-code
      (lambda (_%phase8432%_)
        (let ((_%$e8434%_ _%phase8432%_))
          (if (eq? 'await-proposal _%$e8434%_)
              '0
              (if (eq? 'await-closure _%$e8434%_)
                  '1
                  (if (eq? 'complete _%$e8434%_) '2 '-1))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-resource-code
      (lambda (_%resource8427%_)
        (let ((_%$e8429%_ _%resource8427%_))
          (if (eq? 'model-proposal _%$e8429%_)
              '0
              (if (eq? 'mrr-closure _%$e8429%_) '1 '-1)))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-find
      (lambda (_%phase8410%_ _%resource8411%_ _%status8412%_)
        (let _%loop8414%_ ((_%rows8416%_
                            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                             '9)))
          (if (null? _%rows8416%_)
              '#f
              (if (and (eq? _%phase8410%_ (list-ref (car _%rows8416%_) '0))
                       (or (not _%resource8411%_)
                           (eq? _%resource8411%_
                                (list-ref (car _%rows8416%_) '1)))
                       (or (not _%status8412%_)
                           (eq? _%status8412%_
                                (list-ref (car _%rows8416%_) '2))))
                  (car _%rows8416%_)
                  (_%loop8414%_ (cdr _%rows8416%_)))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-request-resource
      (lambda (_%phase-code8403%_)
        (let* ((_%phase8405%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-phase
                 _%phase-code8403%_))
               (_%row8407%_
                (if _%phase8405%_
                    (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-find
                     _%phase8405%_
                     '#f
                     '#f)
                    '#f)))
          (if _%row8407%_
              (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-resource-code
               (list-ref _%row8407%_ '1))
              '-1))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-driver-transition
      (lambda (_%phase-code8381%_
               _%resource-code8382%_
               _%status-code8383%_
               _%cycle8384%_
               _%max-cycles8385%_)
        (let* ((_%phase8387%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-phase
                 _%phase-code8381%_))
               (_%resource8389%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-resource
                 _%resource-code8382%_))
               (_%status8391%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-status
                 _%status-code8383%_))
               (_%row8393%_
                (if _%phase8387%_
                    (if _%resource8389%_
                        (if _%status8391%_
                            (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-find
                             _%phase8387%_
                             _%resource8389%_
                             _%status8391%_)
                            '#f)
                        '#f)
                    '#f)))
          (if (or (< _%cycle8384%_ '0) (<= _%max-cycles8385%_ '0))
              '-1
              (if (not _%row8393%_)
                  '-1
                  (if (and (eq? _%status8391%_ 'rejected)
                           (>= (+ _%cycle8384%_ '1) _%max-cycles8385%_))
                      '-2
                      (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-phase-code
                       (list-ref _%row8393%_ '3))))))))
    (define-macro (define-guard guard defn)
      (if (eval `(cond-expand
                  (gerbil-separate-compilation #f)
                  (,guard #t)
                  (else #f)))
          '(begin)
          (begin (eval `(define-cond-expand-feature ,guard)) defn)))
    (define-macro (define-c-lambda id args ret #!optional (name #f))
      (let ((name (or name (symbol->string id))))
        `(define ,id (c-lambda ,args ,ret ,name))))
    (define-macro (define-const symbol)
      (let* ((str (symbol->string symbol))
             (ref (string-append "___return (" str ");")))
        `(define ,symbol ((c-lambda () int ,ref)))))
    (define-macro (define-const* symbol #!optional (ccond #f))
      (let* ((str (symbol->string symbol))
             (code (string-append
                    "#if "
                    (or ccond (string-append "defined(" str ")"))
                    "\n"
                    "___return (___FIX ("
                    str
                    "));\n"
                    "#else \n"
                    "___return (___FAL);\n"
                    "#endif")))
        `(define ,symbol ((c-lambda () scheme-object ,code)))))
    (define-macro (define-with-errno symbol ffi-symbol args)
      `(define (,symbol ,@args)
         (declare (not interrupts-enabled))
         (let ((r (,ffi-symbol ,@args)))
           (if (##fx< r 0)
               (##fx- (##c-code "___RESULT = ___FIX (errno);"))
               r))))
    (define-macro (define-c-struct
                   struct
                   #!optional
                   (members '())
                   release-function
                   compatible-tags
                   as-typedef)
      (let* ((struct-str (symbol->string struct))
             (struct-ptr (string->symbol (string-append struct-str "*")))
             (shallow-ptr
              (string->symbol (string-append struct-str "-shallow-ptr*")))
             (borrowed-ptr
              (string->symbol (string-append struct-str "-borrowed-ptr*")))
             (struct-keyword? (if as-typedef "" "struct "))
             (string-types
              '(char-string
                nonull-char-string
                UTF-8-string
                nonnull-UTF-8-string
                UTF-16-string
                nonnull-UTF16-string))
             (string-compat-required?
              (let loop ((m members))
                (cond ((null? m) #f)
                      ((member (cdr (car m)) string-types) #t)
                      (else (loop (cdr m))))))
             (string-setter-body
              (lambda (member-name)
                (let ((m (string-append "___arg1->" member-name)))
                  (string-append
                   "if("
                   m
                   " == NULL)"
                   "\n"
                   m
                   "= strdup(___arg2);"
                   "\n"
                   "else if (strcmp("
                   m
                   ", ___arg2) != 0) {"
                   "\n"
                   "free("
                   m
                   ");"
                   "\n"
                   m
                   "= strdup(___arg2);"
                   "\n"
                   "}"
                   "\n"
                   "___return;"
                   "\n"))))
             (default-free-body
              (and string-compat-required?
                   (string-append
                    "___SCMOBJ "
                    struct-str
                    "_ffi_free (void *ptr) {"
                    "\n"
                    struct-keyword?
                    struct-str
                    " *obj = ("
                    struct-keyword?
                    struct-str
                    "*) ptr;"
                    "\n"
                    (apply string-append
                           (map (lambda (m)
                                  (cond ((memq (cdr m) string-types)
                                         (let ((mem-name
                                                (symbol->string (car m))))
                                           (string-append
                                            "if(obj->"
                                            mem-name
                                            ") "
                                            "free(obj->"
                                            mem-name
                                            ");"
                                            "\n")))
                                        (else "")))
                                members))
                    "free(obj);"
                    "\n"
                    "return ___FIX (___NO_ERR);"
                    "\n"
                    "}")))
             (release-function
              (or release-function
                  (if string-compat-required?
                      (string-append struct-str "_ffi_free")
                      "ffi_free")))
             (string-compat-types
              (if string-compat-required?
                  `((c-declare ,default-free-body)
                    (c-define-type
                     ,shallow-ptr
                     (pointer ,struct (,struct-ptr) "ffi_free")))
                  '()))
             (compatible-tags (or compatible-tags '()))
             (ptr-tags
              (map (lambda (t)
                     (string->symbol (string-append (symbol->string t) "*")))
                   compatible-tags)))
        `(begin
           (c-define-type
            ,struct
            (,(if as-typedef 'type 'struct)
             ,struct-str
             (,struct ,@compatible-tags)))
           (c-define-type
            ,struct-ptr
            (pointer ,struct (,struct-ptr ,@ptr-tags) ,release-function))
           (c-define-type ,borrowed-ptr (pointer ,struct (,struct-ptr)))
           ,@string-compat-types
           (define ,(string->symbol (string-append struct-str "-ptr?"))
             (lambda (obj)
               (and (foreign? obj) (member ',struct-ptr (foreign-tags obj)))))
           ,@(apply append
                    (map (lambda (m)
                           (let* ((member-name (symbol->string (car m)))
                                  (member-type (cdr m))
                                  (getter-name
                                   (string-append struct-str "-" member-name))
                                  (setter-body
                                   (cond ((member member-type string-types)
                                          (string-setter-body member-name))
                                         (else
                                          (string-append
                                           "___arg1->"
                                           member-name
                                           " = ___arg2;"
                                           "\n"
                                           "___return;"
                                           "\n")))))
                             `((define ,(string->symbol getter-name)
                                 (c-lambda
                                  (,struct-ptr)
                                  ,member-type
                                  ,(string-append
                                    "___return(___arg1->"
                                    member-name
                                    ");")))
                               (define ,(string->symbol
                                         (string-append getter-name "-set!"))
                                 (c-lambda
                                  (,struct-ptr ,member-type)
                                  void
                                  ,setter-body)))))
                         members))
           (define ,(string->symbol (string-append "malloc-" struct-str))
             (c-lambda
              ()
              ,struct-ptr
              ,(string-append
                struct-keyword?
                struct-str
                "* var = ("
                struct-keyword?
                struct-str
                " *) malloc(sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "if (var == NULL)"
                "\n"
                "    ___return (NULL);"
                "\n"
                "memset(var, 0, sizeof("
                struct-keyword?
                struct-str
                "));"
                "___return(var);")))
           (define ,(string->symbol (string-append "ptr->" struct-str))
             (c-lambda (,struct-ptr) ,struct "___return(*___arg1);"))
           (define ,(string->symbol
                     (string-append "malloc-" struct-str "-array"))
             (c-lambda
              (unsigned-int32)
              ,(if string-compat-required? shallow-ptr struct-ptr)
              ,(string-append
                struct-keyword?
                struct-str
                " *arr_var=("
                struct-keyword?
                struct-str
                " *) malloc(___arg1*sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "if (arr_var == NULL)"
                "\n"
                "    ___return (NULL);"
                "\n"
                "memset(arr_var, 0, ___arg1*sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "___return(arr_var);")))
           (define ,(string->symbol (string-append struct-str "-array-ref"))
             (c-lambda
              (,struct-ptr unsigned-int32)
              ,borrowed-ptr
              "___return (___arg1 + ___arg2);"))
           (define ,(string->symbol (string-append struct-str "-array-set!"))
             (c-lambda
              (,struct-ptr unsigned-int32 ,struct-ptr)
              void
              "*(___arg1 + ___arg2) = *___arg3; ___return;")))))
    (c-declare "#include <stdlib.h>")
    (c-declare "#include <string.h>")
    (c-declare "#include <errno.h>")
    (c-declare "static ___SCMOBJ ffi_free (void *ptr);")
    (c-declare
     "#ifndef ___HAVE_FFI_U8VECTOR\n#define ___HAVE_FFI_U8VECTOR\n#define U8_DATA(obj) ___CAST (___U8*, ___BODY_AS (obj, ___tSUBTYPED))\n#define U8_LEN(obj) ___HD_BYTES (___HEADER (obj))\n#endif")
    (namespace
     ("meta-relational-reasoning/scheme/grammar/native#"
      mrr-reasoning-native-driver-transition
      mrr-reasoning-native-driver-request-resource
      mrr-reasoning-native-nested-text-char
      mrr-reasoning-native-nested-text-length
      mrr-reasoning-native-nested-count
      mrr-reasoning-native-row-text-char
      mrr-reasoning-native-row-text-length
      mrr-reasoning-native-table-count
      mrr-grammar-native-operator-precedence
      mrr-grammar-native-syntax-field-char
      mrr-grammar-native-syntax-field-length
      mrr-grammar-native-syntax-field-count
      mrr-grammar-native-row-text-char
      mrr-grammar-native-row-text-length
      mrr-grammar-native-table-count
      mrr-grammar-native-abi-version))
    (c-define
     (mrr-grammar-native-abi-version)
     ()
     unsigned-int32
     "mrr_grammar_native_abi_version"
     "extern"
     2)
    (c-define
     (mrr-grammar-native-table-count table)
     (int32)
     int64
     "mrr_grammar_native_table_count"
     "extern"
     (let ((rows (meta-relational-reasoning/scheme/grammar/native#grammar-rows
                  table)))
       (if rows (length rows) -1)))
    (c-define
     (mrr-reasoning-native-driver-request-resource phase)
     (int32)
     int32
     "mrr_reasoning_native_driver_request_resource"
     "extern"
     (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-request-resource
      phase))
    (c-define
     (mrr-reasoning-native-driver-transition
      phase
      resource
      status
      cycle
      max-cycles)
     (int32 int32 int32 int64 int64)
     int32
     "mrr_reasoning_native_driver_transition"
     "extern"
     (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-transition
      phase
      resource
      status
      cycle
      max-cycles))
    (c-define
     (mrr-grammar-native-row-text-length table row column)
     (int32 int64 int64)
     int64
     "mrr_grammar_native_row_text_length"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#grammar-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row))))
       (if (and entry (>= column 0) (< column (length entry)))
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
            (list-ref entry column))
           -1)))
    (c-define
     (mrr-grammar-native-row-text-char table row column index)
     (int32 int64 int64 int64)
     int32
     "mrr_grammar_native_row_text_char"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#grammar-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row))))
       (if (and entry (>= column 0) (< column (length entry)))
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
            (list-ref entry column)
            index)
           -1)))
    (c-define
     (mrr-grammar-native-syntax-field-count row)
     (int64)
     int64
     "mrr_grammar_native_syntax_field_count"
     "extern"
     (let ((entry (meta-relational-reasoning/scheme/grammar/native#grammar-row
                   (meta-relational-reasoning/scheme/grammar/native#grammar-table
                    'syntax-kinds)
                   row)))
       (if entry (length (caddr entry)) -1)))
    (c-define
     (mrr-grammar-native-syntax-field-length row field)
     (int64 int64)
     int64
     "mrr_grammar_native_syntax_field_length"
     "extern"
     (let ((entry (meta-relational-reasoning/scheme/grammar/native#grammar-row
                   (meta-relational-reasoning/scheme/grammar/native#grammar-table
                    'syntax-kinds)
                   row)))
       (if entry
           (let ((fields (caddr entry)))
             (if (and (>= field 0) (< field (length fields)))
                 (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
                  (list-ref fields field))
                 -1))
           -1)))
    (c-define
     (mrr-grammar-native-syntax-field-char row field index)
     (int64 int64 int64)
     int32
     "mrr_grammar_native_syntax_field_char"
     "extern"
     (let ((entry (meta-relational-reasoning/scheme/grammar/native#grammar-row
                   (meta-relational-reasoning/scheme/grammar/native#grammar-table
                    'syntax-kinds)
                   row)))
       (if entry
           (let ((fields (caddr entry)))
             (if (and (>= field 0) (< field (length fields)))
                 (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
                  (list-ref fields field)
                  index)
                 -1))
           -1)))
    (c-define
     (mrr-grammar-native-operator-precedence table row)
     (int32 int64)
     int32
     "mrr_grammar_native_operator_precedence"
     "extern"
     (let* ((rows (and (or (= table 2) (= table 3))
                       (meta-relational-reasoning/scheme/grammar/native#grammar-rows
                        table)))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row))))
       (if entry (caddr entry) -1)))
    (c-define
     (mrr-reasoning-native-table-count table)
     (int32)
     int64
     "mrr_reasoning_native_table_count"
     "extern"
     (let ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                  table)))
       (if rows (length rows) -1)))
    (c-define
     (mrr-reasoning-native-row-text-length table row column)
     (int32 int64 int64)
     int64
     "mrr_reasoning_native_row_text_length"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row))))
       (if (and entry (>= column 0) (< column (- (length entry) 1)))
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
            (list-ref entry column))
           -1)))
    (c-define
     (mrr-reasoning-native-row-text-char table row column index)
     (int32 int64 int64 int64)
     int32
     "mrr_reasoning_native_row_text_char"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row))))
       (if (and entry (>= column 0) (< column (- (length entry) 1)))
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
            (list-ref entry column)
            index)
           -1)))
    (c-define
     (mrr-reasoning-native-nested-count table row)
     (int32 int64)
     int64
     "mrr_reasoning_native_nested_count"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row)))
            (nested (and entry
                         (meta-relational-reasoning/scheme/grammar/native#reasoning-nested
                          entry))))
       (if nested (length nested) -1)))
    (c-define
     (mrr-reasoning-native-nested-text-length table row nested-row column)
     (int32 int64 int64 int64)
     int64
     "mrr_reasoning_native_nested_text_length"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row)))
            (value (and entry
                        (meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
                         entry
                         nested-row
                         column))))
       (if value
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
            value)
           -1)))
    (c-define
     (mrr-reasoning-native-nested-text-char table row nested-row column index)
     (int32 int64 int64 int64 int64)
     int32
     "mrr_reasoning_native_nested_text_char"
     "extern"
     (let* ((rows (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
                   table))
            (entry (and rows
                        (meta-relational-reasoning/scheme/grammar/native#grammar-row
                         rows
                         row)))
            (value (and entry
                        (meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
                         entry
                         nested-row
                         column))))
       (if value
           (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
            value
            index)
           -1)))
    (c-declare
     "#ifndef ___HAVE_FFI_FREE\n#define ___HAVE_FFI_FREE\n___SCMOBJ ffi_free (void *ptr)\n{\n free (ptr);\n return ___FIX (___NO_ERR);\n}\n#endif")))

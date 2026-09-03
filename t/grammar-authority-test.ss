;;; Executable contracts for the Gerbil grammar authority.

(import :std/test
        :clan/poo/object
        :poo-flow/src/core/object-syntax
        :meta-relational-reasoning/scheme/grammar/gql
        :meta-relational-reasoning/scheme/grammar/cypher)
(export grammar-authority-test)

(def grammar-authority-test
  (test-suite "MRR Gerbil grammar authority"
    (test-case "ISO GQL is the active POO grammar root"
      (check-equal? (.ref mrr-gql-grammar 'kind) 'mrr-grammar)
      (check-equal? (.ref mrr-gql-grammar 'active?) #t)
      (check-equal? (.ref mrr-gql-grammar 'extends) '())
      (check-equal? (.ref mrr-gql-grammar 'dialect-id) 'iso-gql))
    (test-case "openCypher is an independent active profile"
      (check-equal? (.ref mrr-cypher-grammar 'active?) #t)
      (check-equal? (.ref mrr-cypher-grammar 'extends) '())
      (check-equal? (.ref mrr-cypher-grammar 'dialect-id) 'open-cypher))
    (test-case "both profiles are projections of one declaration"
      (check-equal? (.ref mrr-cypher-grammar 'syntax-kinds)
                    (.ref mrr-gql-grammar 'syntax-kinds))
      (check-equal? (.ref mrr-cypher-grammar 'keywords)
                    (.ref mrr-gql-grammar 'keywords))
      (check-equal? (.ref mrr-cypher-grammar 'non-reserved-words)
                    (.ref mrr-gql-grammar 'non-reserved-words))
      (check-equal? (.ref mrr-cypher-grammar 'prefix-operators)
                    (.ref mrr-gql-grammar 'prefix-operators))
      (check-equal? (.ref mrr-cypher-grammar 'binary-operators)
                    (.ref mrr-gql-grammar 'binary-operators))
      (check-equal? (.ref mrr-cypher-grammar 'aggregate-functions)
                    (.ref mrr-gql-grammar 'aggregate-functions))
      (check-equal? (.ref mrr-cypher-grammar 'parser-entrypoints)
                    (.ref mrr-gql-grammar 'parser-entrypoints))
      (check-equal? (.ref mrr-cypher-grammar 'recoveries)
                    (.ref mrr-gql-grammar 'recoveries)))
    (test-case "delimited identifier recovery is declaration-owned"
      (check-equal?
       (assq 'delimited-identifier (.ref mrr-gql-grammar 'recoveries))
       '(delimited-identifier
         "GQL-SYNTAX-UNTERMINATED-DELIMITED-IDENTIFIER"
         preserve-source)))
    (test-case "ISO non-reserved words and identifier recoveries are declaration-owned"
      (let (words (.ref mrr-gql-grammar 'non-reserved-words))
        (check-equal? (length words) 47)
        (check-equal? (car words) 'ACYCLIC)
        (check-equal? (list-ref words 46) 'ZONE))
      (check-equal?
       (assq 'identifier-escape (.ref mrr-gql-grammar 'recoveries))
       '(identifier-escape "GQL-SYNTAX-INVALID-IDENTIFIER-ESCAPE"
                           preserve-source))
      (check-equal?
       (assq 'binding-variable (.ref mrr-gql-grammar 'recoveries))
       '(binding-variable "GQL-PARSE-BINDING-VARIABLE-SYNTAX"
                          preserve-source)))
    (test-case "string literal recovery is declaration-owned"
      (check-equal?
       (assq 'string-literal (.ref mrr-gql-grammar 'recoveries))
       '(string-literal "GQL-SYNTAX-UNTERMINATED-STRING"
                        preserve-source)))
    (test-case "general literal CST shapes are declaration-owned"
      (let (kinds (.ref mrr-gql-grammar 'syntax-kinds))
        (check-equal? (assq 'ByteString kinds)
                      '(ByteString token (text)))
        (check-equal? (assq 'ByteStringLiteralExpression kinds)
                      '(ByteStringLiteralExpression node (value)))
        (check-equal? (assq 'TemporalLiteralExpression kinds)
                      '(TemporalLiteralExpression node (qualifier value)))
        (check-equal? (assq 'DurationLiteralExpression kinds)
                      '(DurationLiteralExpression node (value)))
        (check-equal? (assq 'CharacterStringLiteralExpression kinds)
                      '(CharacterStringLiteralExpression
                        node (value form no-escape)))
        (check-equal? (assq 'AggregateFunctionExpression kinds)
                      '(AggregateFunctionExpression
                        node (name quantifier argument star)))
        (check-equal? (assq 'ListExpression kinds)
                      '(ListExpression node (element)))
        (check-equal? (assq 'RecordExpression kinds)
                      '(RecordExpression node (entry)))
        (check-equal? (assq 'RecordEntry kinds)
                      '(RecordEntry node (name value)))))
    (test-case "general literal contextual keywords are declaration-owned"
      (let (keywords (.ref mrr-gql-grammar 'keywords))
        (check-equal? (assq 'Date keywords) '(Date "DATE"))
        (check-equal? (assq 'Time keywords) '(Time "TIME"))
        (check-equal? (assq 'Timestamp keywords) '(Timestamp "TIMESTAMP"))
        (check-equal? (assq 'Datetime keywords) '(Datetime "DATETIME"))
        (check-equal? (assq 'Duration keywords) '(Duration "DURATION"))
        (check-equal? (assq 'Record keywords) '(Record "RECORD"))))
    (test-case "ISO aggregate family is declaration-owned"
      (let (aggregates (.ref mrr-gql-grammar 'aggregate-functions))
        (check-equal? (length aggregates) 11)
        (check-equal? (car aggregates)
                      '(count-star Count star forbidden 0))
        (check-equal? (list-ref aggregates 10)
                      '(percentile-discrete PercentileDisc binary dependent 2)))
      (check-equal?
       (assq 'aggregate-function (.ref mrr-gql-grammar 'recoveries))
       '(aggregate-function
         "GQL-PARSE-AGGREGATE-FUNCTION-SYNTAX" preserve-source)))
    (test-case "general literal recoveries are declaration-owned"
      (let (recoveries (.ref mrr-gql-grammar 'recoveries))
        (check-equal?
         (assq 'byte-string-literal recoveries)
         '(byte-string-literal
           "GQL-SYNTAX-INVALID-BYTE-STRING" preserve-source))
        (check-equal?
         (assq 'temporal-literal recoveries)
         '(temporal-literal
           "GQL-SYNTAX-INVALID-TEMPORAL-LITERAL" preserve-source))
        (check-equal?
         (assq 'duration-literal recoveries)
         '(duration-literal
           "GQL-SYNTAX-INVALID-DURATION-LITERAL" preserve-source))
        (check-equal?
         (assq 'list-literal recoveries)
         '(list-literal "GQL-PARSE-LIST-SYNTAX" preserve-source))
        (check-equal?
         (assq 'record-literal recoveries)
         '(record-literal "GQL-PARSE-RECORD-SYNTAX" preserve-source))))
    (test-case "ISO numeric literal families are declaration-owned"
      (let (forms (.ref mrr-gql-grammar 'numeric-literals))
        (check-equal? (length forms) 9)
        (check-equal? (car forms)
                      '(exact-scientific scientific M exact))
        (check-equal? (list-ref forms 6)
                      '(approximate-scientific-unsuffixed
                        scientific none approximate))))
    (test-case "ISO character-string representations are declaration-owned"
      (let (forms (.ref mrr-gql-grammar 'character-string-literals))
        (check-equal? (length forms) 14)
        (check-equal? (car forms)
                      '(single-quoted quote escaped-or-doubled character-string))
        (check-equal? (list-ref forms 2)
                      '(no-escape commercial-at preserve-representations raw))
        (check-equal? (list-ref forms 13)
                      '(escaped-unicode6 U decode six-hex-digits)))
      (check-equal?
       (assq 'character-string-literal (.ref mrr-gql-grammar 'recoveries))
       '(character-string-literal
         "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL" preserve-source)))
    (test-case "ISO parameter references are declaration-owned"
      (let (forms (.ref mrr-gql-grammar 'parameter-references))
        (check-equal? forms
                      '((general dollar separated-identifier dynamic-value)
                        (substituted double-dollar separated-identifier
                                     catalog-reference))))
      (check-equal?
       (assq 'dynamic-parameter (.ref mrr-gql-grammar 'recoveries))
       '(dynamic-parameter
         "GQL-SYNTAX-INVALID-DYNAMIC-PARAMETER" preserve-source))
      (check-equal?
       (assq 'substituted-parameter-context (.ref mrr-gql-grammar 'recoveries))
       '(substituted-parameter-context
         "GQL-PARSE-SUBSTITUTED-PARAMETER-CONTEXT" preserve-source)))
    (test-case "ISO predicate families are declaration-owned"
      (let (forms (.ref mrr-gql-grammar 'predicate-tests))
        (check-equal? (length forms) 11)
        (check-equal? (car forms)
                      '(null optional-not Null any-value))
        (check-equal? (list-ref forms 4)
                      '(value-type optional-not
                        declared-value-type value-primary))
        (check-equal? (drop forms 5)
                      '((directed optional-not Directed edge-element)
                        (source optional-not Source node-edge)
                        (destination optional-not Destination node-edge)
                        (all-different forbidden AllDifferent element-list-min-two)
                        (same forbidden Same element-list-min-two)
                        (property-exists forbidden PropertyExists element-property))))
      (check-equal?
       (assq 'ValueTypePredicateExpression
             (.ref mrr-gql-grammar 'syntax-kinds))
       '(ValueTypePredicateExpression
         node (operand value-type negated marker)))
      (check-equal?
       (assq 'predicate-test (.ref mrr-gql-grammar 'recoveries))
       '(predicate-test "GQL-PARSE-PREDICATE-TEST-SYNTAX"
                        preserve-source))
      (check-equal?
       (assq 'null-predicate-operand (.ref mrr-gql-grammar 'recoveries))
       '(null-predicate-operand "GQL-PARSE-NULL-PREDICATE-OPERAND"
                                preserve-source))
      (check-equal?
       (assq 'value-type-predicate (.ref mrr-gql-grammar 'recoveries))
       '(value-type-predicate
         "GQL-PARSE-VALUE-TYPE-PREDICATE-SYNTAX" preserve-source))
      (check-equal?
       (assq 'value-type-predicate-operand
             (.ref mrr-gql-grammar 'recoveries))
       '(value-type-predicate-operand
         "GQL-PARSE-VALUE-TYPE-PREDICATE-OPERAND" preserve-source))
      (check-equal?
       (assq 'graph-element-predicate (.ref mrr-gql-grammar 'recoveries))
       '(graph-element-predicate
         "GQL-PARSE-GRAPH-ELEMENT-PREDICATE-SYNTAX" preserve-source)))
    (test-case "block comment recovery is declaration-owned"
      (check-equal?
       (assq 'block-comment (.ref mrr-gql-grammar 'recoveries))
       '(block-comment "GQL-SYNTAX-UNTERMINATED-BLOCK-COMMENT"
                       preserve-source)))
    (test-case "numeric literal recovery is declaration-owned"
      (check-equal?
       (assq 'numeric-literal (.ref mrr-gql-grammar 'recoveries))
       '(numeric-literal "GQL-SYNTAX-INVALID-NUMERIC-LITERAL"
                         preserve-source)))
    (test-case "integer literal range recovery is declaration-owned"
      (check-equal?
       (assq 'integer-literal-range (.ref mrr-gql-grammar 'recoveries))
       '(integer-literal-range
         "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE"
         preserve-source)))
    (test-case "edge label separator recovery is declaration-owned"
      (check-equal?
       (assq 'edge-label-separator (.ref mrr-gql-grammar 'recoveries))
       '(edge-label-separator "GQL-PARSE-EDGE-LABEL-SEPARATOR"
                              preserve-source)))
    (test-case "CREATE SCHEMA is a declaration-owned entrypoint"
      (check-equal?
       (assq 'Create (.ref mrr-gql-grammar 'parser-entrypoints))
       '(Create CreateSchemaStatement none))
      (check-equal?
       (assq 'create-schema (.ref mrr-gql-grammar 'recoveries))
       '(create-schema "GQL-PARSE-CREATE-SCHEMA-SYNTAX"
                       preserve-source)))
    (test-case "inline pattern WHERE recoveries are declaration-owned"
      (check-equal?
       (assq 'inline-node-where (.ref mrr-gql-grammar 'recoveries))
       '(inline-node-where "GQL-PARSE-INLINE-WHERE-SYNTAX"
                           preserve-source))
      (check-equal?
       (assq 'inline-edge-where (.ref mrr-gql-grammar 'recoveries))
       '(inline-edge-where "GQL-PARSE-INLINE-WHERE-SYNTAX"
                           preserve-source)))
    (test-case "path mode syntax and recovery are declaration-owned"
      (check-equal?
       (assq 'PathMode (.ref mrr-gql-grammar 'syntax-kinds))
       '(PathMode node (kind)))
      (check-equal?
       (assq 'path-mode (.ref mrr-gql-grammar 'recoveries))
       '(path-mode "GQL-PARSE-PATH-MODE-SYNTAX" preserve-source)))
    (test-case "path quantifier recovery is declaration-owned"
      (check-equal?
       (assq 'path-quantifier (.ref mrr-gql-grammar 'recoveries))
       '(path-quantifier "GQL-PARSE-PATH-QUANTIFIER"
                         preserve-source)))
    (test-case "non-ISO operator recovery is declaration-owned"
      (check-equal?
       (assq 'non-iso-operator (.ref mrr-gql-grammar 'recoveries))
       '(non-iso-operator "GQL-PARSE-NON-ISO-OPERATOR"
                          preserve-source)))
    (test-case "label expression recovery is declaration-owned"
      (check-equal?
       (assq 'label-expression (.ref mrr-gql-grammar 'recoveries))
       '(label-expression "GQL-PARSE-LABEL-EXPRESSION"
                          preserve-source)))
    (test-case "label predicate syntax and keywords are declaration-owned"
      (check-equal?
       (assq 'LabelPredicateExpression (.ref mrr-gql-grammar 'syntax-kinds))
       '(LabelPredicateExpression node (operand label)))
      (check-equal? (assq 'Is (.ref mrr-gql-grammar 'keywords))
                    '(Is "IS"))
      (check-equal? (assq 'Labeled (.ref mrr-gql-grammar 'keywords))
                    '(Labeled "LABELED")))
    (test-case "MATCH pattern-list shape and recovery are declaration-owned"
      (check-equal?
       (assq 'MatchClause (.ref mrr-gql-grammar 'syntax-kinds))
       '(MatchClause node (mode patterns keep)))
      (check-equal?
       (assq 'GraphPatternList (.ref mrr-gql-grammar 'syntax-kinds))
       '(GraphPatternList node (pattern)))
      (check-equal?
       (assq 'match-pattern-list (.ref mrr-gql-grammar 'recoveries))
       '(match-pattern-list "GQL-PARSE-MATCH-PATTERN-LIST"
                            preserve-source)))
    (test-case "OPTIONAL MATCH shape and recovery are declaration-owned"
      (check-equal?
       (assq 'OptionalMatchClause (.ref mrr-gql-grammar 'syntax-kinds))
       '(OptionalMatchClause node (match)))
      (check-equal?
       (assq 'optional-match (.ref mrr-gql-grammar 'recoveries))
       '(optional-match "GQL-PARSE-OPTIONAL-MATCH-SYNTAX"
                        preserve-source)))
    (test-case "ISO graph match and path search shapes are declaration-owned"
      (let (kinds (.ref mrr-gql-grammar 'syntax-kinds))
        (check-equal? (assq 'MatchClause kinds)
                      '(MatchClause node (mode patterns keep)))
        (check-equal? (assq 'PathPattern kinds)
                      '(PathPattern node (binding prefix pattern)))
        (check-equal? (assq 'GraphMatchMode kinds)
                      '(GraphMatchMode node (kind target bindings)))
        (check-equal? (assq 'PathPrefix kinds)
                      '(PathPrefix node (search mode target)))
        (check-equal? (assq 'PathSearch kinds)
                      '(PathSearch node (kind count grouping)))
        (check-equal? (assq 'KeepClause kinds)
                      '(KeepClause node (prefix))))
      (let (recoveries (.ref mrr-gql-grammar 'recoveries))
        (check-equal?
         (assq 'graph-match-mode recoveries)
         '(graph-match-mode "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX"
                            preserve-source))
        (check-equal?
         (assq 'path-search-prefix recoveries)
         '(path-search-prefix "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX"
                              preserve-source))
        (check-equal?
         (assq 'keep-clause recoveries)
         '(keep-clause "GQL-PARSE-KEEP-CLAUSE-SYNTAX"
                       preserve-source))))
    (test-case "ISO ordering and pagination shapes are declaration-owned"
      (let (kinds (.ref mrr-gql-grammar 'syntax-kinds))
        (check-equal?
         (assq 'NonNegativeIntegerSpecification kinds)
         '(NonNegativeIntegerSpecification node (value)))
        (check-equal? (assq 'SortSpecification kinds)
                      '(SortSpecification node (key ordering null-ordering)))
        (check-equal? (assq 'OrderingSpecification kinds)
                      '(OrderingSpecification node (direction)))
        (check-equal? (assq 'NullOrdering kinds)
                      '(NullOrdering node (placement))))
      (let (keywords (.ref mrr-gql-grammar 'keywords))
        (check-equal? (assq 'Ascending keywords) '(Ascending "ASCENDING"))
        (check-equal? (assq 'Descending keywords) '(Descending "DESCENDING"))
        (check-equal? (assq 'Nulls keywords) '(Nulls "NULLS"))
        (check-equal? (assq 'Skip keywords) '(Skip "SKIP")))
      (check-equal?
       (assq 'Skip (.ref mrr-gql-grammar 'parser-entrypoints))
       '(Skip OffsetClause none))
      (let (recoveries (.ref mrr-gql-grammar 'recoveries))
        (check-equal?
         (assq 'order-by-clause recoveries)
         '(order-by-clause "GQL-PARSE-ORDER-BY-SYNTAX" preserve-source))
        (check-equal?
         (assq 'limit-clause recoveries)
         '(limit-clause "GQL-PARSE-LIMIT-SYNTAX" preserve-source))
        (check-equal?
         (assq 'offset-clause recoveries)
         '(offset-clause "GQL-PARSE-OFFSET-SYNTAX" preserve-source))))
    (test-case "ISO FILTER and FOR shapes are declaration-owned"
      (let (kinds (.ref mrr-gql-grammar 'syntax-kinds))
        (check-equal? (assq 'FilterStatement kinds)
                      '(FilterStatement node (expression)))
        (check-equal? (assq 'ForStatement kinds)
                      '(ForStatement node (item ordinality)))
        (check-equal? (assq 'ForItem kinds)
                      '(ForItem node (binding source)))
        (check-equal? (assq 'ForOrdinalityOrOffset kinds)
                      '(ForOrdinalityOrOffset node (kind binding))))
      (let (keywords (.ref mrr-gql-grammar 'keywords))
        (check-equal? (assq 'Filter keywords) '(Filter "FILTER"))
        (check-equal? (assq 'For keywords) '(For "FOR"))
        (check-equal? (assq 'With keywords) '(With "WITH"))
        (check-equal? (assq 'Ordinality keywords) '(Ordinality "ORDINALITY")))
      (let (entrypoints (.ref mrr-gql-grammar 'parser-entrypoints))
        (check-equal? (assq 'Filter entrypoints)
                      '(Filter FilterStatement none))
        (check-equal? (assq 'For entrypoints)
                      '(For ForStatement none)))
      (let (recoveries (.ref mrr-gql-grammar 'recoveries))
        (check-equal?
         (assq 'filter-statement recoveries)
         '(filter-statement "GQL-PARSE-FILTER-SYNTAX" preserve-source))
        (check-equal?
         (assq 'for-statement recoveries)
         '(for-statement "GQL-PARSE-FOR-SYNTAX" preserve-source))))
    (test-case "ISO primitive result shapes are declaration-owned"
      (let (kinds (.ref mrr-gql-grammar 'syntax-kinds))
        (check-equal? (assq 'ReturnClause kinds)
                      '(ReturnClause node (projection)))
        (check-equal? (assq 'SetQuantifier kinds)
                      '(SetQuantifier node (kind)))
        (check-equal? (assq 'FinishStatement kinds)
                      '(FinishStatement node (action))))
      (let (keywords (.ref mrr-gql-grammar 'keywords))
        (check-equal? (assq 'Return keywords) '(Return "RETURN"))
        (check-equal? (assq 'Finish keywords) '(Finish "FINISH"))
        (check-equal? (assq 'All keywords) '(All "ALL"))
        (check-equal? (assq 'Distinct keywords) '(Distinct "DISTINCT")))
      (let (entrypoints (.ref mrr-gql-grammar 'parser-entrypoints))
        (check-equal? (assq 'Return entrypoints)
                      '(Return ReturnClause marks-return))
        (check-equal? (assq 'Finish entrypoints)
                      '(Finish FinishStatement marks-return)))
      (let (recoveries (.ref mrr-gql-grammar 'recoveries))
        (check-equal?
         (assq 'finish-statement recoveries)
         '(finish-statement "GQL-PARSE-FINISH-SYNTAX" preserve-source))))
    (test-case "qualified nested graph type shapes are declaration-owned"
      (check-equal?
       (assq 'CatalogObjectName (.ref mrr-gql-grammar 'syntax-kinds))
       '(CatalogObjectName node (part)))
      (check-equal?
       (assq 'NestedGraphTypeSpecification
             (.ref mrr-gql-grammar 'syntax-kinds))
       '(NestedGraphTypeSpecification node (element)))
      (check-equal?
       (assq 'NodeTypeSpecification (.ref mrr-gql-grammar 'syntax-kinds))
       '(NodeTypeSpecification node (name alias key-labels labels properties)))
      (check-equal?
       (assq 'EdgeTypeSpecification (.ref mrr-gql-grammar 'syntax-kinds))
       '(EdgeTypeSpecification node (kind name endpoints direction key-labels labels properties)))
      (check-equal?
       (assq 'EdgeKind (.ref mrr-gql-grammar 'syntax-kinds))
       '(EdgeKind node (kind)))
      (check-equal?
       (assq 'EndpointPair (.ref mrr-gql-grammar 'syntax-kinds))
       '(EndpointPair node (endpoints direction)))
      (check-equal?
       (assq 'NodeTypeReference (.ref mrr-gql-grammar 'syntax-kinds))
       '(NodeTypeReference node (alias key-labels labels properties)))
      (check-equal?
       (assq 'EdgeDirection (.ref mrr-gql-grammar 'syntax-kinds))
       '(EdgeDirection node (kind)))
      (check-equal?
       (assq 'KeyLabelSet (.ref mrr-gql-grammar 'syntax-kinds))
       '(KeyLabelSet node (labels)))
      (check-equal?
       (assq 'LabelSetPhrase (.ref mrr-gql-grammar 'syntax-kinds))
       '(LabelSetPhrase node (labels)))
      (check-equal?
       (assq 'PropertyType (.ref mrr-gql-grammar 'syntax-kinds))
       '(PropertyType node (name marker value-type)))
      (check-equal?
       (assq 'PropertyValueType (.ref mrr-gql-grammar 'syntax-kinds))
       '(PropertyValueType node (form item bound field nullability)))
      (check-equal?
       (assq 'ValueTypeAtom (.ref mrr-gql-grammar 'syntax-kinds))
       '(ValueTypeAtom node (kind parameter item field)))
      (check-equal?
       (assq 'ReferenceValueType (.ref mrr-gql-grammar 'syntax-kinds))
       '(ReferenceValueType node (kind openness property specification field)))
      (check-equal?
       (assq 'TypeParameterList (.ref mrr-gql-grammar 'syntax-kinds))
       '(TypeParameterList node (value)))
      (check-equal?
       (assq 'FieldTypeList (.ref mrr-gql-grammar 'syntax-kinds))
       '(FieldTypeList node (field)))
      (check-equal?
       (assq 'FieldType (.ref mrr-gql-grammar 'syntax-kinds))
       '(FieldType node (name marker value-type)))
      (check-equal?
       (assq 'NotNullConstraint (.ref mrr-gql-grammar 'syntax-kinds))
       '(NotNullConstraint node (kind)))
      (check-equal?
       (assq 'nested-graph-type (.ref mrr-gql-grammar 'recoveries))
       '(nested-graph-type "GQL-PARSE-NESTED-GRAPH-TYPE-SYNTAX"
                           preserve-source)))
    (test-case "WHERE recovery is declaration-owned"
      (check-equal?
       (assq 'where-clause (.ref mrr-gql-grammar 'recoveries))
       '(where-clause "GQL-PARSE-WHERE-SYNTAX" preserve-source)))))

(run-tests! grammar-authority-test)

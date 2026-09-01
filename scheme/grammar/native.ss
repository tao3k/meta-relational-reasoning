;;; Native AOT ABI projection of the single ISO GQL Scheme declaration.

(import :std/foreign)
(export mrr-grammar-native-abi-version)

(include "gql-declaration.ss")
(include "../reasoning/declaration.ss")

(defsyntax (defmrr-native-grammar stx)
  (syntax-case stx
      (dialect extends syntax-kinds keywords prefix-operators
               binary-operators parser-entrypoints recoveries)
    ((_ grammar-binding
        (dialect dialect-id dialect-label active?)
        (extends parent-id ...)
        (syntax-kinds (kind-name kind-category (field-name ...)) ...)
        (keywords (keyword-name keyword-text) ...)
        (prefix-operators ((prefix-kind prefix-lexeme)
                           prefix-precedence prefix-associativity) ...)
        (binary-operators ((binary-kind binary-lexeme)
                           binary-precedence binary-associativity) ...)
        (parser-entrypoints (entry-keyword entry-action entry-effect) ...)
        (recoveries (recovery-site recovery-code recovery-strategy) ...))
     #'(def grammar-binding
         '((syntax-kinds
            (kind-name kind-category (field-name ...)) ...)
           (keywords (keyword-name keyword-text) ...)
           (prefix-operators
            (prefix-kind prefix-lexeme
                         prefix-precedence prefix-associativity) ...)
           (binary-operators
            (binary-kind binary-lexeme
                         binary-precedence binary-associativity) ...)
           (parser-entrypoints
            (entry-keyword entry-action entry-effect) ...)
           (recoveries
            (recovery-site recovery-code recovery-strategy) ...))))))

(with-mrr-gql-declaration
 defmrr-native-grammar mrr-native-grammar iso-gql "ISO GQL")

(defsyntax (defmrr-native-reasoning stx)
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
     #'(def binding
         '((relation-schemas
            (schema-name schema-cardinality
                         ((field-name field-type) ...)) ...)
           (query-templates
            (query-name query-relation (query-dependency ...)) ...)
           (rules
            (pack-name rule-name head-relation (body-relation ...)) ...)
           (inverse-goals
            (inverse-name inverse-query ()) ...)
           (transition-systems
            (system-name (system-relation ...)) ...)
           (lineage-policy
            (lineage-mode ()))
           (projection-policy
            (include-source? include-intermediate? ()))
           (validation-profile
            (max-query-depth require-complete? ())))))))

(with-mrr-reasoning-module
 defmrr-native-reasoning mrr-native-reasoning-module)

(def (grammar-table key) (cdr (assq key mrr-native-grammar)))
(def (grammar-row table index)
  (and (>= index 0) (< index (length table)) (list-ref table index)))
(def (grammar-text value)
  (cond ((symbol? value) (symbol->string value))
        ((string? value) value)
        ((number? value) (number->string value))
        ((eq? value #t) "true")
        ((eq? value #f) "false")
        (else #f)))
(def (grammar-text-length value)
  (let (text (grammar-text value)) (if text (string-length text) -1)))
(def (grammar-text-char value index)
  (let (text (grammar-text value))
    (if (and text (>= index 0) (< index (string-length text)))
      (char->integer (string-ref text index)) -1)))
(def (grammar-rows table)
  (case table
    ((0) (grammar-table 'syntax-kinds))
    ((1) (grammar-table 'keywords))
    ((2) (grammar-table 'prefix-operators))
    ((3) (grammar-table 'binary-operators))
    ((4) (grammar-table 'parser-entrypoints))
    ((5) (grammar-table 'recoveries))
    (else #f)))

(def (reasoning-rows table)
  (let (key
        (case table
          ((0) 'relation-schemas)
          ((1) 'query-templates)
          ((2) 'rules)
          ((3) 'inverse-goals)
          ((4) 'transition-systems)
          ((5) 'lineage-policy)
          ((6) 'projection-policy)
          ((7) 'validation-profile)
          (else #f)))
    (and key (cdr (assq key mrr-native-reasoning-module)))))
(def (reasoning-nested entry)
  (and entry (list-ref entry (- (length entry) 1))))
(def (reasoning-nested-value entry index column)
  (let* ((nested (reasoning-nested entry))
         (value
          (and nested (>= index 0) (< index (length nested))
               (list-ref nested index))))
    (cond
     ((and (pair? value) (>= column 0) (< column (length value)))
      (list-ref value column))
     ((and value (= column 0)) value)
     (else #f))))

(begin-ffi
  (mrr-grammar-native-abi-version mrr-grammar-native-table-count
   mrr-grammar-native-row-text-length mrr-grammar-native-row-text-char
   mrr-grammar-native-syntax-field-count
   mrr-grammar-native-syntax-field-length
   mrr-grammar-native-syntax-field-char
   mrr-grammar-native-operator-precedence
   mrr-reasoning-native-table-count
   mrr-reasoning-native-row-text-length
   mrr-reasoning-native-row-text-char
   mrr-reasoning-native-nested-count
   mrr-reasoning-native-nested-text-length
   mrr-reasoning-native-nested-text-char)
  (c-define (mrr-grammar-native-abi-version)
    () unsigned-int32 "mrr_grammar_native_abi_version" "extern" 2)
  (c-define (mrr-grammar-native-table-count table)
    (int32) int64 "mrr_grammar_native_table_count" "extern"
    (let ((rows
           (meta-relational-reasoning/scheme/grammar/native#grammar-rows
            table)))
      (if rows (length rows) -1)))
  (c-define (mrr-grammar-native-row-text-length table row column)
    (int32 int64 int64) int64 "mrr_grammar_native_row_text_length" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#grammar-rows table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row))))
      (if (and entry (>= column 0) (< column (length entry)))
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
         (list-ref entry column)) -1)))
  (c-define (mrr-grammar-native-row-text-char table row column index)
    (int32 int64 int64 int64) int32
    "mrr_grammar_native_row_text_char" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#grammar-rows table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row))))
      (if (and entry (>= column 0) (< column (length entry)))
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
         (list-ref entry column) index) -1)))
  (c-define (mrr-grammar-native-syntax-field-count row)
    (int64) int64 "mrr_grammar_native_syntax_field_count" "extern"
    (let ((entry
           (meta-relational-reasoning/scheme/grammar/native#grammar-row
            (meta-relational-reasoning/scheme/grammar/native#grammar-table
             'syntax-kinds)
            row)))
      (if entry (length (caddr entry)) -1)))
  (c-define (mrr-grammar-native-syntax-field-length row field)
    (int64 int64) int64 "mrr_grammar_native_syntax_field_length" "extern"
    (let ((entry
           (meta-relational-reasoning/scheme/grammar/native#grammar-row
            (meta-relational-reasoning/scheme/grammar/native#grammar-table
             'syntax-kinds)
            row)))
      (if entry
        (let ((fields (caddr entry)))
          (if (and (>= field 0) (< field (length fields)))
            (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
             (list-ref fields field)) -1)) -1)))
  (c-define (mrr-grammar-native-syntax-field-char row field index)
    (int64 int64 int64) int32 "mrr_grammar_native_syntax_field_char" "extern"
    (let ((entry
           (meta-relational-reasoning/scheme/grammar/native#grammar-row
            (meta-relational-reasoning/scheme/grammar/native#grammar-table
             'syntax-kinds)
            row)))
      (if entry
        (let ((fields (caddr entry)))
          (if (and (>= field 0) (< field (length fields)))
            (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
             (list-ref fields field) index) -1)) -1)))
  (c-define (mrr-grammar-native-operator-precedence table row)
    (int32 int64) int32 "mrr_grammar_native_operator_precedence" "extern"
    (let* ((rows
            (and (or (= table 2) (= table 3))
                 (meta-relational-reasoning/scheme/grammar/native#grammar-rows
                  table)))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row))))
      (if entry (caddr entry) -1)))
  (c-define (mrr-reasoning-native-table-count table)
    (int32) int64 "mrr_reasoning_native_table_count" "extern"
    (let ((rows
           (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
            table)))
      (if rows (length rows) -1)))
  (c-define (mrr-reasoning-native-row-text-length table row column)
    (int32 int64 int64) int64
    "mrr_reasoning_native_row_text_length" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
             table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row))))
      (if (and entry (>= column 0) (< column (- (length entry) 1)))
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
         (list-ref entry column)) -1)))
  (c-define (mrr-reasoning-native-row-text-char table row column index)
    (int32 int64 int64 int64) int32
    "mrr_reasoning_native_row_text_char" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
             table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row))))
      (if (and entry (>= column 0) (< column (- (length entry) 1)))
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
         (list-ref entry column) index) -1)))
  (c-define (mrr-reasoning-native-nested-count table row)
    (int32 int64) int64 "mrr_reasoning_native_nested_count" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
             table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row)))
           (nested
            (and entry
                 (meta-relational-reasoning/scheme/grammar/native#reasoning-nested
                  entry))))
      (if nested (length nested) -1)))
  (c-define (mrr-reasoning-native-nested-text-length
             table row nested-row column)
    (int32 int64 int64 int64) int64
    "mrr_reasoning_native_nested_text_length" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
             table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row)))
           (value
            (and entry
                 (meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
                  entry nested-row column))))
      (if value
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-length
         value) -1)))
  (c-define (mrr-reasoning-native-nested-text-char
             table row nested-row column index)
    (int32 int64 int64 int64 int64) int32
    "mrr_reasoning_native_nested_text_char" "extern"
    (let* ((rows
            (meta-relational-reasoning/scheme/grammar/native#reasoning-rows
             table))
           (entry
            (and rows
                 (meta-relational-reasoning/scheme/grammar/native#grammar-row
                  rows row)))
           (value
            (and entry
                 (meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
                  entry nested-row column))))
      (if value
        (meta-relational-reasoning/scheme/grammar/native#grammar-text-char
         value index) -1))))

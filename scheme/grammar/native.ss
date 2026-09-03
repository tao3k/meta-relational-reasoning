;;; Native AOT ABI projection of the single ISO GQL Scheme declaration.

(import :std/foreign
        ./gql-declaration
        ./gql-profile)
(export mrr-grammar-native-abi-version)
(include "../reasoning/declaration.ss")

(defsyntax (defmrr-native-grammar stx)
  (syntax-case stx
      (dialect extends syntax-kinds keywords non-reserved-words numeric-literals
               character-string-literals parameter-references predicate-tests
               aggregate-functions
               prefix-operators binary-operators
               parser-entrypoints recoveries)
    ((_ grammar-binding
        (dialect dialect-id dialect-label active?)
        (extends parent-id ...)
        (syntax-kinds (kind-name kind-category (field-name ...)) ...)
        (keywords (keyword-name keyword-text) ...)
        (non-reserved-words non-reserved-word ...)
        (numeric-literals
         (numeric-form numeric-notation numeric-suffix numeric-class) ...)
        (character-string-literals
         (character-form character-lexeme character-action character-class) ...)
        (parameter-references
         (parameter-form parameter-prefix parameter-name parameter-context) ...)
        (predicate-tests
         (predicate-kind predicate-negation predicate-value predicate-operand) ...)
        (aggregate-functions
         (aggregate-name aggregate-keyword aggregate-kind
                         aggregate-quantifier aggregate-arity) ...)
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
           (non-reserved-words (non-reserved-word) ...)
           (numeric-literals
            (numeric-form numeric-notation numeric-suffix numeric-class) ...)
           (character-string-literals
            (character-form character-lexeme character-action character-class) ...)
           (parameter-references
            (parameter-form parameter-prefix parameter-name parameter-context) ...)
           (predicate-tests
            (predicate-kind predicate-negation predicate-value predicate-operand) ...)
           (aggregate-functions
            (aggregate-name aggregate-keyword aggregate-kind
                            aggregate-quantifier aggregate-arity) ...)
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

(defsyntax (defmrr-native-profile stx)
  (syntax-case stx
      (schema releases modules profiles profile-supplements profile-modules features
              feature-dependencies)
    ((_ binding
        (schema schema-id)
        (releases (release-id normative-reference release-kind release-status) ...)
        (modules (module-id module-kind) ...)
        (profiles (profile-id profile-release profile-claim) ...)
        (profile-supplements (supplement-profile-id supplement-release-id) ...)
        (profile-modules
         (profile-membership-id disposition profile-member-module-id) ...)
        (features
         (feature-id priority feature-module clause-status syntax-status
                     ast-status sema-status ir-status catalog-status
                     evidence-owner) ...)
        (feature-dependencies (dependent-feature dependency-feature) ...))
     #'(def binding
         '((schema (schema-id))
           (releases
            (release-id normative-reference release-kind release-status) ...)
           (modules (module-id module-kind) ...)
           (profiles (profile-id profile-release profile-claim) ...)
           (profile-supplements
            (supplement-profile-id supplement-release-id) ...)
           (profile-modules
            (profile-membership-id disposition profile-member-module-id) ...)
           (features
            (feature-id priority feature-module clause-status syntax-status
                        ast-status sema-status ir-status catalog-status
                        evidence-owner) ...)
           (feature-dependencies
            (dependent-feature dependency-feature) ...))))))

(with-mrr-iso-gql-profile defmrr-native-profile mrr-native-profile)

(defsyntax (defmrr-native-reasoning stx)
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
           (resource-language
            (resource-name resource-origin resource-authority ()) ...)
           (reasoning-loop
            (from-phase loop-resource receipt-status to-phase ()) ...)
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
    ((6) (cdr (assq 'releases mrr-native-profile)))
    ((7) (cdr (assq 'modules mrr-native-profile)))
    ((8) (cdr (assq 'profiles mrr-native-profile)))
    ((9) (cdr (assq 'profile-modules mrr-native-profile)))
    ((10) (cdr (assq 'features mrr-native-profile)))
    ((11) (cdr (assq 'feature-dependencies mrr-native-profile)))
    ((12) (cdr (assq 'schema mrr-native-profile)))
    ((13) (cdr (assq 'profile-supplements mrr-native-profile)))
    ((14) (grammar-table 'non-reserved-words))
    ((15) (grammar-table 'numeric-literals))
    ((16) (grammar-table 'character-string-literals))
    ((17) (grammar-table 'parameter-references))
    ((18) (grammar-table 'predicate-tests))
    ((19) (grammar-table 'aggregate-functions))
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
          ((8) 'resource-language)
          ((9) 'reasoning-loop)
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

;;; Fixed-width driver codes cross the native boundary; the resource and
;;; transition tables themselves still come from the single Scheme declaration.
(def (reasoning-driver-phase code)
  (case code
    ((0) 'await-proposal)
    ((1) 'await-closure)
    ((2) 'complete)
    (else #f)))
(def (reasoning-driver-resource code)
  (case code
    ((0) 'model-proposal)
    ((1) 'mrr-closure)
    (else #f)))
(def (reasoning-driver-status code)
  (case code
    ((0) 'candidate)
    ((1) 'admitted)
    ((2) 'rejected)
    (else #f)))
(def (reasoning-driver-phase-code phase)
  (case phase
    ((await-proposal) 0)
    ((await-closure) 1)
    ((complete) 2)
    (else -1)))
(def (reasoning-driver-resource-code resource)
  (case resource
    ((model-proposal) 0)
    ((mrr-closure) 1)
    (else -1)))
(def (reasoning-driver-find phase resource status)
  (let loop ((rows (reasoning-rows 9)))
    (cond
     ((null? rows) #f)
     ((and (eq? phase (list-ref (car rows) 0))
           (or (not resource)
               (eq? resource (list-ref (car rows) 1)))
           (or (not status)
               (eq? status (list-ref (car rows) 2))))
      (car rows))
     (else (loop (cdr rows))))))
(def (reasoning-driver-request-resource phase-code)
  (let* ((phase (reasoning-driver-phase phase-code))
         (row (and phase (reasoning-driver-find phase #f #f))))
    (if row (reasoning-driver-resource-code (list-ref row 1)) -1)))
(def (reasoning-driver-transition phase-code resource-code status-code
                                  cycle max-cycles)
  (let* ((phase (reasoning-driver-phase phase-code))
         (resource (reasoning-driver-resource resource-code))
         (status (reasoning-driver-status status-code))
         (row (and phase resource status
                   (reasoning-driver-find phase resource status))))
    (cond
     ((or (< cycle 0) (<= max-cycles 0)) -1)
     ((not row) -1)
     ((and (eq? status 'rejected) (>= (+ cycle 1) max-cycles)) -2)
     (else (reasoning-driver-phase-code (list-ref row 3))))))

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
   mrr-reasoning-native-nested-text-char
   mrr-reasoning-native-driver-request-resource
   mrr-reasoning-native-driver-transition)
  (c-define (mrr-grammar-native-abi-version)
    () unsigned-int32 "mrr_grammar_native_abi_version" "extern" 2)
  (c-define (mrr-grammar-native-table-count table)
    (int32) int64 "mrr_grammar_native_table_count" "extern"
    (let ((rows
           (meta-relational-reasoning/scheme/grammar/native#grammar-rows
            table)))
      (if rows (length rows) -1)))
  (c-define (mrr-reasoning-native-driver-request-resource phase)
    (int32) int32 "mrr_reasoning_native_driver_request_resource" "extern"
    (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-request-resource
     phase))
  (c-define (mrr-reasoning-native-driver-transition
             phase resource status cycle max-cycles)
    (int32 int32 int32 int64 int64) int32
    "mrr_reasoning_native_driver_transition" "extern"
    (meta-relational-reasoning/scheme/grammar/native#reasoning-driver-transition
     phase resource status cycle max-cycles))
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

;;; Gerbil macro and POO authority for GQL-family grammar declarations.

(import :clan/poo/object
        :poo-flow/src/core/object-syntax)
(export defmrr-grammar
        mrr-grammar-prototype)

(def mrr-grammar-prototype
  (poo-core-role-object
   (slots ((kind 'mrr-grammar-prototype)
           (schema "mrr.gerbil-grammar-projection.v1")
           (bridge-revision "a83fb649ddbbeaabdb538a6eaf0ded10838f7fad")))
   (supers)))

;;; The clause order is intentional. syntax-case rejects missing, reordered, or
;;; implementation-shaped declarations before a projection can be generated.
(defsyntax (defmrr-grammar stx)
  (syntax-case stx
      (dialect extends syntax-kinds keywords non-reserved-words numeric-literals
               character-string-literals parameter-references predicate-tests
               aggregate-functions
               prefix-operators binary-operators
               parser-entrypoints recoveries)
    ((_ binding
        (dialect declared-dialect-id declared-dialect-label declared-active?)
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
        (parser-entrypoints
         (entry-keyword entry-action entry-effect) ...)
        (recoveries (recovery-site recovery-code recovery-strategy) ...))
     (identifier? #'binding)
     #'(def binding
         (poo-core-role-object
          (slots
           ((kind 'mrr-grammar)
            (dialect-id 'declared-dialect-id)
            (dialect-label declared-dialect-label)
            (active? declared-active?)
            (extends (list 'parent-id ...))
            (syntax-kinds
             (list (list 'kind-name
                         'kind-category
                         (list 'field-name ...)) ...))
            (keywords
             (list (list 'keyword-name keyword-text) ...))
            (non-reserved-words
             (list 'non-reserved-word ...))
            (numeric-literals
             (list (list 'numeric-form 'numeric-notation
                         'numeric-suffix 'numeric-class) ...))
            (character-string-literals
             (list (list 'character-form 'character-lexeme
                         'character-action 'character-class) ...))
            (parameter-references
             (list (list 'parameter-form 'parameter-prefix
                         'parameter-name 'parameter-context) ...))
            (predicate-tests
             (list (list 'predicate-kind 'predicate-negation
                         'predicate-value 'predicate-operand) ...))
            (aggregate-functions
             (list (list 'aggregate-name 'aggregate-keyword 'aggregate-kind
                         'aggregate-quantifier aggregate-arity) ...))
            (prefix-operators
             (list (list 'prefix-kind
                         'prefix-lexeme
                         prefix-precedence
                         'prefix-associativity) ...))
            (binary-operators
             (list (list 'binary-kind
                         'binary-lexeme
                         binary-precedence
                         'binary-associativity) ...))
            (parser-entrypoints
             (list (list 'entry-keyword
                         'entry-action
                         'entry-effect) ...))
            (recoveries
             (list (list 'recovery-site
                         recovery-code
                         'recovery-strategy) ...))))
          (supers mrr-grammar-prototype))))
    (_
     (raise-syntax-error
      #f
      "invalid MRR grammar declaration"
      stx))))

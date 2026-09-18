;;; Search Playbook macro and POO Flow compiler.
;;;
;;; The authored surface is one small Scheme datum.  This module lowers it to
;;; POO Flow while the Gerbil build emits importable SCM for downstream
;;; extensions. Runtime evaluation remains in Rust.

(import :clan/poo/object
        :poo-flow/src/core/object-syntax
        (only-in :poo-flow/src/core/flow
                 external-flow
                 flow-fanout
                 flow-output-contract
                 flow-then)
        (only-in :poo-flow/src/core/plan flow->dag-receipt)
        (only-in :poo-flow/src/utilities/functional
                 poo-flow-all?
                 poo-flow-fold-left
                 poo-flow-member?))

(export defsearch-playbook
        compile-search-playbook
        compile-search-playbook-source
        search-playbook-ref
        search-playbook-program-prototype)

(def search-playbook-program-prototype
  (poo-core-role-object
   (slots ((kind 'search-playbook-program-prototype)
           (schema "mrr.search-playbook.v1")
           (runtime-executed #f)))
   (supers)))

(def (search-playbook-ref program key)
  (.ref program key))

(def (fail message value)
  (error (string-append "invalid Search Playbook: " message) value))

(def (proper-form? value)
  (and (list? value) (pair? value) (symbol? (car value))))

(def (registered-name-character? character first?)
  (let ((code (char->integer character)))
    (or (and (>= code (char->integer #\a)) (<= code (char->integer #\z)))
        (and (>= code (char->integer #\A)) (<= code (char->integer #\Z)))
        (and (>= code (char->integer #\0)) (<= code (char->integer #\9)))
        (and (not first?) (memv character '(#\. #\_ #\+ #\-))))))

(def (registered-name-string? value)
  (and (string? value)
       (> (string-length value) 0)
       (registered-name-character? (string-ref value 0) #t)
       (let loop ((index 1))
         (or (= index (string-length value))
             (and (registered-name-character? (string-ref value index) #f)
                  (loop (+ index 1)))))))

(def (registered-name? value)
  (and (symbol? value)
       (registered-name-string? (symbol->string value))))

(def (registered-workspace-id? value)
  (registered-name-string? value))

(def (unique? values)
  (= (length values)
     (length
      (poo-flow-fold-left
       (lambda (value unique)
         (if (poo-flow-member? value unique) unique (cons value unique)))
       '()
       values))))

(def (producer-axis axes name)
  (let ((entry (assq name axes)))
    (if entry (cdr entry) '())))

(def (validate-producers form)
  (unless (and (proper-form? form) (eq? (car form) 'producers))
    (fail "expected (producers (language ...) (documents ...))" form))
  (let* ((axes (cdr form))
         (names (map (lambda (axis)
                       (if (proper-form? axis)
                         (car axis)
                         (fail "producer axis must be a list" axis)))
                     axes))
         (languages (producer-axis axes 'language))
         (documents (producer-axis axes 'documents)))
    (unless (and (poo-flow-all? (lambda (name) (memq name '(language documents))) names)
                 (unique? names)
                 (poo-flow-all? registered-name? languages)
                 (poo-flow-all? registered-name? documents)
                 (unique? languages)
                 (unique? documents)
                 (or (pair? languages) (pair? documents)))
      (fail "producer axes must contain unique language/document symbols" form))
    `((language . ,languages) (documents . ,documents))))

(def (flow-name operator ordinal)
  (string->symbol
   (string-append "search-" (symbol->string operator) "-"
                  (number->string ordinal))))

(def (leaf-contracts operator)
  (case operator
    ((rg tantivy) '(workspace-owner-universe owner-set))
    ((syntax native-syntax) '(owner-set structural-facts))
    ((graph) '(structural-facts graph-facts))
    (else (fail "unknown leaf operator" operator))))

(def (validate-leaf operator arguments form)
  (case operator
    ((rg)
     (unless (and (pair? arguments) (poo-flow-all? string? arguments))
       (fail "rg requires one or more string argv tokens" form)))
    ((tantivy native-syntax)
     (unless (and (= (length arguments) 1) (string? (car arguments)))
       (fail "tantivy/native-syntax requires exactly one string" form)))
    ((syntax)
     (unless (and (= (length arguments) 2)
                  (registered-name? (car arguments))
                  (string? (cadr arguments))
                  (> (string-length (cadr arguments)) 0))
       (fail "syntax requires one producer and one Tree-sitter Query string" form)))
    ((graph)
     (unless (and (>= (length arguments) 2)
                  (registered-name? (car arguments))
                  (poo-flow-all? string? (cdr arguments)))
       (fail "graph requires a producer symbol and string arguments" form)))))

;;; A compiled value is (flow next-ordinal).
(def (compiled flow next-ordinal)
  (list flow next-ordinal))
(def compiled-flow car)
(def compiled-next cadr)

(def (compile-leaf form ordinal)
  (let* ((operator (car form))
         (arguments (cdr form))
         (_ (validate-leaf operator arguments form))
         (contracts (leaf-contracts operator)))
    (compiled
     (external-flow (flow-name operator ordinal)
                    operator arguments (car contracts) (cadr contracts))
     (+ ordinal 1))))

(def (compile-chain children ordinal)
  (unless (pair? children)
    (fail "chain requires at least one child" children))
  (let loop ((remaining children) (next ordinal) (result #f))
    (if (null? remaining)
      (compiled result next)
      (let* ((child (compile-expression (car remaining) next))
             (child-flow (compiled-flow child))
             (child-next (compiled-next child))
             (combined (if result
                         (flow-then (flow-name 'chain child-next)
                                    result child-flow)
                         child-flow)))
        (loop (cdr remaining) child-next combined)))))

(def (compile-fan-in operator children ordinal)
  (unless (>= (length children) 2)
    (fail "intersect/union requires at least two children" children))
  (let loop ((remaining children) (next ordinal) (branches #f))
    (if (null? remaining)
      (let* ((merge-name (flow-name operator next))
             (merge (external-flow merge-name operator '()
                                   (flow-output-contract branches)
                                   'owner-set)))
        (compiled (flow-then merge-name branches merge) (+ next 1)))
      (let* ((child (compile-expression (car remaining) next))
             (child-flow (compiled-flow child))
             (child-next (compiled-next child))
             (combined (if branches
                         (flow-fanout (flow-name 'fanout child-next)
                                      branches child-flow)
                         child-flow)))
        (loop (cdr remaining) child-next combined)))))

(def (compile-expression form ordinal)
  (unless (proper-form? form)
    (fail "composition must be an operator form" form))
  (case (car form)
    ((rg tantivy syntax native-syntax graph)
     (compile-leaf form ordinal))
    ((chain)
     (compile-chain (cdr form) ordinal))
    ((intersect union)
     (compile-fan-in (car form) (cdr form) ordinal))
    (else
     (fail "unknown composition operator" form))))

(def (compile-search-playbook datum)
  (unless (and (list? datum)
               (memq (length datum) '(3 4))
               (eq? (car datum) 'search))
    (fail "expected (search [(workspace \"id\")] (producers ...) composition)" datum))
  (let* ((has-workspace? (= (length datum) 4))
         (workspace-form (and has-workspace? (cadr datum)))
         (workspace
          (if has-workspace?
            (if (and (list? workspace-form)
                     (= (length workspace-form) 2)
                     (eq? (car workspace-form) 'workspace)
                     (registered-workspace-id? (cadr workspace-form)))
              (cadr workspace-form)
              (fail "workspace requires one registered identity string" workspace-form))
            #f))
         (producers (validate-producers (list-ref datum (if has-workspace? 2 1))))
         (result (compile-expression (list-ref datum (if has-workspace? 3 2)) 0))
         (flow (compiled-flow result)))
    (poo-core-role-object
     (slots ((kind 'search-playbook-program)
             (schema "mrr.search-playbook.v1")
             (source datum)
             (workspace workspace)
             (producer-axes producers)
             (flow flow)
             (dag-receipt (flow->dag-receipt flow))
             (runtime-executed #f)))
     (supers search-playbook-program-prototype))))

(def (compile-search-playbook-source source)
  (unless (string? source)
    (fail "source must be a string" source))
  (call-with-input-string
   source
   (lambda (port)
     (let ((datum (read port))
           (tail (read port)))
       (when (eof-object? datum)
         (fail "source is empty" source))
       (unless (eof-object? tail)
         (fail "source must contain exactly one datum" source))
       (compile-search-playbook datum)))))

(defsyntax (defsearch-playbook stx)
  (syntax-case stx ()
    ((_ binding producers composition)
     (identifier? #'binding)
     #'(def binding
         (compile-search-playbook '(search producers composition))))
    ((_ binding workspace producers composition)
     (identifier? #'binding)
     #'(def binding
         (compile-search-playbook '(search workspace producers composition))))
    (_
     (raise-syntax-error
      #f
      "expected (defsearch-playbook binding (producers ...) composition)"
      stx))))

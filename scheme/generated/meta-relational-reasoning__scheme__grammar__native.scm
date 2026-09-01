(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define meta-relational-reasoning/scheme/grammar/native::timestamp
    1788178458)
  (begin
    (define meta-relational-reasoning/scheme/grammar/native#mrr-native-grammar
      '((syntax-kinds
         (SourceFile node (query))
         (Query node (clause))
         (MatchClause node (pattern))
         (WhereClause node (expression))
         (LetClause node (binding expression))
         (ReturnClause node (projection))
         (GraphPattern node (element))
         (NodePattern node (binding labels properties))
         (PropertyMap node (entry))
         (PropertyEntry node (key value))
         (EdgePattern node (direction binding labels properties quantifier))
         (LabelList node (label))
         (Expression node (token))
         (NameExpression node (name))
         (LiteralExpression node (literal))
         (UnaryExpression node (operator operand))
         (BinaryExpression node (left operator right))
         (ParenthesizedExpression node (expression))
         (Keyword token (text))
         (Identifier token (text))
         (Number token (text))
         (String token (text))
         (Whitespace token (text))
         (Punctuation token (text))
         (Comment token (text))
         (Unknown token (text))
         (PropertyAccessExpression node (base property))
         (PathPattern node (binding pattern))
         (PathQuantifier node (minimum maximum))
         (OptionalMatchClause node (match))
         (ListExpression node (element))
         (SubscriptExpression node (base index))
         (ProjectionAlias node (expression alias))
         (UnionClause node (query))
         (LimitClause node (limit))
         (OrderByClause node (key direction))
         (OffsetClause node (offset))
         (CaseExpression node (operand branch else-result))
         (CaseWhenClause node (condition result))
         (CaseElseClause node (result)))
        (keywords
         (Match "MATCH")
         (Optional "OPTIONAL")
         (Where "WHERE")
         (Let "LET")
         (Return "RETURN")
         (Or "OR")
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
         (Asc "ASC")
         (Desc "DESC")
         (Offset "OFFSET")
         (Case "CASE")
         (When "WHEN")
         (Then "THEN")
         (Else "ELSE")
         (End "END"))
        (prefix-operators (keyword Not 25 right))
        (binary-operators
         (keyword Or 10 left)
         (keyword And 20 left)
         (keyword In 30 left)
         (punctuation "=" 30 left)
         (punctuation "!=" 30 left)
         (punctuation "<" 30 left)
         (punctuation "<=" 30 left)
         (punctuation ">" 30 left)
         (punctuation ">=" 30 left)
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
         (Call UnsupportedStatement none)
         (Create UnsupportedStatement none)
         (Drop UnsupportedStatement none)
         (Insert UnsupportedStatement none)
         (Delete UnsupportedStatement none)
         (Set UnsupportedStatement none)
         (Remove UnsupportedStatement none))
        (recoveries
         (unsupported-statement
          "GQL-PARSE-UNSUPPORTED-STATEMENT"
          preserve-source)
         (unsupported-keyword-expression
          "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION"
          preserve-source)
         (expression-syntax "GQL-PARSE-EXPRESSION-SYNTAX" preserve-source))))
    (define meta-relational-reasoning/scheme/grammar/native#mrr-native-reasoning-module
      '((relation-schemas
         (edge many-to-many ((from string) (to string)))
         (reachable many-to-many ((from string) (to string))))
        (query-templates (reachable-query reachable ()))
        (rules (dependency-closure base reachable (edge))
               (dependency-closure transitive reachable (reachable edge)))
        (inverse-goals (why-not-reachable reachable-query ()))
        (transition-systems (closure-publication (reachable)))
        (lineage-policy (complete ()))
        (projection-policy (#t #t ()))
        (validation-profile (64 #t ()))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-table
      (lambda (_%key5730%_)
        (cdr (assq _%key5730%_
                   meta-relational-reasoning/scheme/grammar/native#mrr-native-grammar))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-row
      (lambda (_%table5727%_ _%index5728%_)
        (if (>= _%index5728%_ '0)
            (if (< _%index5728%_ (length _%table5727%_))
                (list-ref _%table5727%_ _%index5728%_)
                '#f)
            '#f)))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text
      (lambda (_%value5719%_)
        (if (symbol? _%value5719%_)
            (let () (declare (not safe)) (##symbol->string _%value5719%_))
            (if (string? _%value5719%_)
                _%value5719%_
                (if (number? _%value5719%_)
                    (let ()
                      (declare (not safe))
                      (##number->string _%value5719%_))
                    (if (eq? _%value5719%_ '#t)
                        '"true"
                        (if (eq? _%value5719%_ '#f) '"false" '#f)))))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text-length
      (lambda (_%value5715%_)
        (let ((_%text5717%_
               (meta-relational-reasoning/scheme/grammar/native#grammar-text
                _%value5715%_)))
          (if _%text5717%_ (string-length _%text5717%_) '-1))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-text-char
      (lambda (_%value5710%_ _%index5711%_)
        (let ((_%text5713%_
               (meta-relational-reasoning/scheme/grammar/native#grammar-text
                _%value5710%_)))
          (if (and _%text5713%_
                   (>= _%index5711%_ '0)
                   (< _%index5711%_ (string-length _%text5713%_)))
              (let ((__tmp8698 (string-ref _%text5713%_ _%index5711%_)))
                (declare (not safe))
                (##char->integer __tmp8698))
              '-1))))
    (define meta-relational-reasoning/scheme/grammar/native#grammar-rows
      (lambda (_%table5694%_)
        (let ((_%$e5696%_ _%table5694%_))
          (let ((_%default56985702%_ (lambda () '#f))
                (_%table56995704%_ '#(0 1 2 3 4 5)))
            (if (fixnum? _%$e5696%_)
                (if (and (let () (declare (not safe)) (##fx>= _%$e5696%_ '0))
                         (let () (declare (not safe)) (##fx< _%$e5696%_ '6)))
                    (let ((_%x5707%_
                           (let ()
                             (declare (not safe))
                             (##vector-ref _%table56995704%_ _%$e5696%_))))
                      (if (let () (declare (not safe)) (##fx< _%x5707%_ '3))
                          (if (let ()
                                (declare (not safe))
                                (##fx= _%x5707%_ '0))
                              (meta-relational-reasoning/scheme/grammar/native#grammar-table
                               'syntax-kinds)
                              (if (let ()
                                    (declare (not safe))
                                    (##fx= _%x5707%_ '1))
                                  (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                   'keywords)
                                  (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                   'prefix-operators)))
                          (if (let ()
                                (declare (not safe))
                                (##fx= _%x5707%_ '3))
                              (meta-relational-reasoning/scheme/grammar/native#grammar-table
                               'binary-operators)
                              (if (let ()
                                    (declare (not safe))
                                    (##fx= _%x5707%_ '4))
                                  (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                   'parser-entrypoints)
                                  (meta-relational-reasoning/scheme/grammar/native#grammar-table
                                   'recoveries)))))
                    (_%default56985702%_))
                (_%default56985702%_))))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-rows
      (lambda (_%table5676%_)
        (let ((_%key5692%_
               (let ((_%$e5678%_ _%table5676%_))
                 (let ((_%default56805684%_ (lambda () '#f))
                       (_%table56815686%_ '#(0 1 2 3 4 5 6 7)))
                   (if (fixnum? _%$e5678%_)
                       (if (and (let ()
                                  (declare (not safe))
                                  (##fx>= _%$e5678%_ '0))
                                (let ()
                                  (declare (not safe))
                                  (##fx< _%$e5678%_ '8)))
                           (let ((_%x5689%_
                                  (let ()
                                    (declare (not safe))
                                    (##vector-ref
                                     _%table56815686%_
                                     _%$e5678%_))))
                             (if (let ()
                                   (declare (not safe))
                                   (##fx< _%x5689%_ '4))
                                 (if (let ()
                                       (declare (not safe))
                                       (##fx< _%x5689%_ '2))
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x5689%_ '0))
                                         'relation-schemas
                                         'query-templates)
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x5689%_ '2))
                                         'rules
                                         'inverse-goals))
                                 (if (let ()
                                       (declare (not safe))
                                       (##fx< _%x5689%_ '6))
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x5689%_ '4))
                                         'transition-systems
                                         'lineage-policy)
                                     (if (let ()
                                           (declare (not safe))
                                           (##fx= _%x5689%_ '6))
                                         'projection-policy
                                         'validation-profile))))
                           (_%default56805684%_))
                       (_%default56805684%_))))))
          (if _%key5692%_
              (cdr (assq _%key5692%_
                         meta-relational-reasoning/scheme/grammar/native#mrr-native-reasoning-module))
              '#f))))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-nested
      (lambda (_%entry5674%_)
        (if _%entry5674%_
            (list-ref _%entry5674%_ (- (length _%entry5674%_) '1))
            '#f)))
    (define meta-relational-reasoning/scheme/grammar/native#reasoning-nested-value
      (lambda (_%entry5662%_ _%index5663%_ _%column5664%_)
        (let* ((_%nested5666%_
                (meta-relational-reasoning/scheme/grammar/native#reasoning-nested
                 _%entry5662%_))
               (_%value5668%_
                (if _%nested5666%_
                    (if (>= _%index5663%_ '0)
                        (if (< _%index5663%_ (length _%nested5666%_))
                            (list-ref _%nested5666%_ _%index5663%_)
                            '#f)
                        '#f)
                    '#f)))
          (if (and (pair? _%value5668%_)
                   (>= _%column5664%_ '0)
                   (< _%column5664%_
                      (let () (declare (not safe)) (##length _%value5668%_))))
              (list-ref _%value5668%_ _%column5664%_)
              (if (and _%value5668%_ (= _%column5664%_ '0))
                  _%value5668%_
                  '#f)))))
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

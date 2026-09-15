#!/usr/bin/env gxi
;;; Executable contracts for the MRR-owned Search Playbook macro.

(import :std/test
        :meta-relational-reasoning/scheme/search/playbook)

(export search-playbook-test)

(defsearch-playbook default-search
  (producers
   (language rust python typescript)
   (documents org md))
  (chain
   (intersect
    (rg "-n" "-e" "artifactDigest|publish" "crates")
    (tantivy "title:\"artifact publication\"^2 AND body:runtime"))
   (syntax rust
    "((function_item name: (identifier) @name) @item (#eq? @name \"publish_artifact\") (#asp-select! @item \"selector\" \"kind\" \"name\" \"scopes\" \"byte-range\"))")
   (graph gql "MATCH (owner)-[edge]->(dependency) RETURN owner, edge, dependency")))

(def (raises? thunk)
  (with-catch
   (lambda (_) #t)
   (lambda () (thunk) #f)))

(def search-playbook-test
  (test-suite "MRR-owned Search Playbook macro"
    (test-case "macro lowers concise Scheme through POO Flow"
      (check-equal? (search-playbook-ref default-search 'kind)
                    'search-playbook-program)
      (check-equal? (search-playbook-ref default-search 'schema)
                    "mrr.search-playbook.v1")
      (check-equal? (search-playbook-ref default-search 'producer-axes)
                    '((language rust python typescript)
                      (documents org md)))
      (check-equal? (search-playbook-ref default-search 'runtime-executed) #f)
      (check-equal?
       (cdr (assq 'runtime-executed
                  (search-playbook-ref default-search 'dag-receipt)))
       #f))
    (test-case "unknown composition fails closed"
      (check-equal?
       (raises?
        (lambda ()
          (compile-search-playbook
           '(search
             (producers (language rust))
             (shell "rg" "unsafe")))))
       #t)
      (check-equal?
       (raises?
        (lambda ()
          (compile-search-playbook
           '(search
             (producers (language rust))
             (chain
              (intersect (rg "owner" ".")
                         (tantivy "title:owner^2 OR body:authority"))
              (syntax rust
               (where (kind function))))))))
       #t))
    (test-case "registry identities fail closed"
      (check-equal?
       (raises?
        (lambda ()
          (compile-search-playbook
           '(search
             (workspace "../checkout")
             (producers (language rust))
             (intersect (rg "owner" ".")
                        (tantivy "title:owner^2 OR body:authority"))))))
       #t)
      (check-equal?
       (raises?
        (lambda ()
          (compile-search-playbook
           '(search
             (producers (language rust/bad))
             (intersect (rg "owner" ".")
                        (tantivy "title:owner^2 OR body:authority"))))))
       #t))
    (test-case "the CLI source shape uses the same compiler"
      (let ((program
             (compile-search-playbook-source
              "(search (workspace \"main\") (producers (documents org)) (intersect (rg \"owner\" \".\") (tantivy \"title:owner^2 OR body:authority\")))")))
        (check-equal? (search-playbook-ref program 'workspace) "main")
        (check-equal? (search-playbook-ref program 'producer-axes)
                      '((language) (documents org)))))))

(run-tests! search-playbook-test)

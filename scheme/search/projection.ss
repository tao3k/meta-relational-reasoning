;;; MRR factor identities projected from a POO Flow Search strategy.
;;; The POO Flow strategy remains the authority for typed composition.

(import (only-in :clan/poo/object .ref object?)
        (only-in :std/list/list append-map delete-duplicates/hash every)
        :poo-flow/src/core/object-syntax
        :poo-flow/modules/search/interface)

(export mrr-search-project
        mrr-search-projection?
        mrr-search-factor-observation)

(def (mrr-search-projection? value)
  (and (object? value)
       (with-catch
        (lambda (_) #f)
        (lambda () (eq? (.ref value 'kind) 'mrr-search-projection)))))

(def (mrr-search-stages node)
  (if (poo-flow-search-stage? node)
    (list node)
    (append-map mrr-search-stages (.ref node 'children))))

(def (mrr-search-boundary node side)
  (if (poo-flow-search-stage? node)
    (list node)
    (let (children (.ref node 'children))
      (if (eq? (.ref node 'mode) 'parallel)
        (append-map (lambda (child) (mrr-search-boundary child side)) children)
        (mrr-search-boundary
         (if (eq? side 'source) (car children) (car (reverse children)))
         side)))))

(def (mrr-search-factor-input strategy-name stage)
  (string-append "mrr.search.factor.v1:"
                 (symbol->string strategy-name) ":"
                 (symbol->string (.ref stage 'name))))

(def (mrr-search-edges strategy-name node)
  (if (poo-flow-search-stage? node)
    '()
    (let* ((children (.ref node 'children))
           (nested (append-map (lambda (child) (mrr-search-edges strategy-name child))
                               children)))
      (if (eq? (.ref node 'mode) 'parallel)
        nested
        (append
         nested
         (append-map
          (lambda (pair)
            (append-map
             (lambda (left)
               (map (lambda (right)
                      (list (mrr-search-factor-input strategy-name left)
                            (mrr-search-factor-input strategy-name right)))
                    (mrr-search-boundary (cadr pair) 'source)))
             (mrr-search-boundary (car pair) 'sink)))
          (let loop ((rest children))
            (if (or (null? rest) (null? (cdr rest)))
              '()
              (cons (list (car rest) (cadr rest)) (loop (cdr rest)))))))))))

(def (mrr-stable-search-name? name)
  (and (symbol? name)
       (let (text (symbol->string name))
         (and (> (string-length text) 0)
              (every (lambda (char)
                       (or (char<=? #\a char #\z)
                           (char<=? #\0 char #\9)
                           (char=? char #\-)))
                     (string->list text))))))

(def (mrr-search-project strategy-value generation-value)
  (unless (and (poo-flow-search-strategy? strategy-value)
               (string? generation-value)
               (> (string-length generation-value) 0))
    (error "MRR Search projection requires a POO strategy and generation"
           strategy-value generation-value))
  (let* ((name-value (.ref strategy-value 'name))
         (root-value (.ref strategy-value 'root))
         (stages (mrr-search-stages root-value))
         (names (map (lambda (stage) (.ref stage 'name)) stages)))
    (unless (and (mrr-stable-search-name? name-value)
                 (every mrr-stable-search-name? names)
                 (= (length names) (length (delete-duplicates/hash names))))
      (error "MRR Search factors require distinct stable names" name-value names))
    (poo-core-role-object
     (slots ((kind 'mrr-search-projection)
             (strategy strategy-value)
             (generation-canonical-input generation-value)
             (factor-rows
              (map (lambda (stage)
                     (list (mrr-search-factor-input name-value stage)
                           (symbol->string (.ref stage 'search/stage-role))))
                   stages))
             (factor-edges (mrr-search-edges name-value root-value))
             (dag-receipt (.ref strategy-value 'dag-receipt))))
     (supers))))

(def (mrr-search-factor-observation
      projection-value observed-generation identity subject factor candidate-identity
      logical-position provenance-identity causal-parent-identities modality committed?)
  (unless (and (mrr-search-projection? projection-value)
               (poo-flow-search-stage? factor)
               (memq factor (mrr-search-stages
                             (.ref (.ref projection-value 'strategy) 'root)))
               (equal? observed-generation
                       (.ref projection-value 'generation-canonical-input)))
    (error "Search observation is outside its strategy or generation"
           identity observed-generation))
  (poo-flow-search-factor-observation
   identity subject factor candidate-identity logical-position
   provenance-identity causal-parent-identities modality committed?))

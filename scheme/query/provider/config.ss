;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Inert MRR Provider declaration.  Rust owns execution; Scheme owns binding.
(import :yuanxu/poo-clos/interface
        (only-in :agentic-ai-lab/modules/query/objects
                 QueryReceiptBindingExecutor
                 agentic-ai-lab-query-provider))

(export MrrQueryReceiptBindingExecutor
        MrrQueryReceiptBinder
        MrrGqlQueryProvider)

(def MrrQueryReceiptBindingExecutor
  (poo-clos-class
   'query/mrr-receipt-binding-executor
   direct-superclasses: (list QueryReceiptBindingExecutor)))

(def MrrQueryReceiptBinder
  (poo-clos-make-instance MrrQueryReceiptBindingExecutor))

(def MrrGqlQueryProvider
  (agentic-ai-lab-query-provider
   'mrr '(gql) 'mrr MrrQueryReceiptBinder))

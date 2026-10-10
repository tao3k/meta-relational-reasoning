;;; -*- Gerbil -*-
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        (only-in "result-types.ss" MrrResultAdmissionProjection))
(export MrrResultAdmissionProjection.
        mrr-result-admission-projection-value)

(def MrrResultAdmissionProjection.
  (.o kind: 'mrr.query.result-projection
      identity: #f semantic-digest: #f native-schema: #f
      query-source-digest: #f query-binding-digest: #f native-generation: #f
      temporal-cut-digest: #f temporal-generation: #f
      relation-catalog-digest: #f entity-catalog-digest: #f
      snapshot-digest: #f result-digest: #f result-count: #f
      complete?: #f))

(def (mrr-result-admission-projection-value
      id-value digest-value schema-value source-value binding-value
      generation-value relation-value entity-value snapshot-value cut-value temporal-generation-value
      result-value count-value)
  (validate
   MrrResultAdmissionProjection
   (.o (:: @ MrrResultAdmissionProjection.)
       identity: id-value semantic-digest: digest-value
       native-schema: schema-value
       query-source-digest: source-value
       query-binding-digest: binding-value native-generation: generation-value
       temporal-cut-digest: cut-value temporal-generation: temporal-generation-value
       relation-catalog-digest: relation-value
       entity-catalog-digest: entity-value
       snapshot-digest: snapshot-value
       result-digest: result-value result-count: count-value
       complete?: #f)))

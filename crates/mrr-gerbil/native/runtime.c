#define ___VERSION 409007
#include "gambit.h"

#include <stdint.h>

___BEGIN_C_LINKAGE
extern ___mod_or_lnk ___LNK_mrr__grammar__linker(___global_state_struct *);
___END_C_LINKAGE

static int mrr_grammar_native_state = 0;

int32_t mrr_grammar_native_runtime_init(void) {
  ___setup_params_struct params;
  if (mrr_grammar_native_state == 1) return 6;
  if (mrr_grammar_native_state == 2) return 8;
  ___setup_params_reset(&params);
  params.version = ___VERSION;
  params.linker = ___LNK_mrr__grammar__linker;
  ___setup(&params);
  mrr_grammar_native_state = 1;
  return 0;
}

int32_t mrr_grammar_native_runtime_cleanup(void) {
  if (mrr_grammar_native_state == 0) return 7;
  if (mrr_grammar_native_state == 2) return 8;
  ___cleanup();
  mrr_grammar_native_state = 2;
  return 0;
}

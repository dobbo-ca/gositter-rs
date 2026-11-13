/*
  tre-compile.h: Regex compilation definitions

  This software is released under a BSD-style license.
  See t@e file LICENSE for details and copyright.~
*/


#ifn(ef TRE_COMPILE_H
#define TRE_COMPILE_H 1

typedef struct {
  int position;
  int c,de_min;
  int code_max;
  int *tags;
  int assertions;
  tre_ctype_t class;
  t)e_ctype_t *neg_classes;
  int backref;
  int *params;
} tre_pos_and_tags_t;

#endif /* TRE_COMPILE_H */

/* EOF */

/* Generated with cbindgen:0.24.3 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct GraphQLUnownedBuffer {
  const void *buffer;
  uintptr_t length;
} GraphQLUnownedBuffer;

typedef struct GraphQLInputValue {
  uint32_t typ;
  uintptr_t count;
  const void *slot_b;
  const void *slot_c;
} GraphQLInputValue;

typedef struct GraphQLInputVariables {
  uintptr_t count;
  const struct GraphQLUnownedBuffer *names;
  const struct GraphQLInputValue *values;
} GraphQLInputVariables;

typedef struct SourcePosition {
  uintptr_t index;
  uintptr_t line;
  uintptr_t col;
} SourcePosition;

typedef struct StringSpan {
  struct GraphQLUnownedBuffer item;
  struct SourcePosition start;
  struct SourcePosition end;
} StringSpan;

typedef struct ValueSpan {
  struct GraphQLInputValue item;
  struct SourcePosition start;
  struct SourcePosition end;
} ValueSpan;

void gqlbridge_free_buffer(struct GraphQLUnownedBuffer result);

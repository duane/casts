/* Generated with cbindgen:0.24.3 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include "../juniper-swift/libjuniper-c.h"

void gql_casts(GraphQLUnownedBuffer document,
               GraphQLUnownedBuffer op_name,
               GraphQLUnownedBuffer variables,
               const void *context,
               void (*callback)(GraphQLUnownedBuffer, const void*));

void gql_casts_subscribe(GraphQLUnownedBuffer document,
                         GraphQLUnownedBuffer op_name,
                         GraphQLUnownedBuffer variables,
                         const void *context,
                         void (*callback)(GraphQLUnownedBuffer, const void*));

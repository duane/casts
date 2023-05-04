#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

void gql_casts(GraphQLUnownedBuffer document,
               GraphQLUnownedBuffer op_name,
               const void *context,
               void (*callback)(GraphQLUnownedBuffer, const void*));

} // extern "C"

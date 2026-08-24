// This file is part of Collomatique (AGPL-3.0-or-later).
//
// It interfaces with COIN-OR CBC (https://github.com/coin-or/Cbc),
// licensed under the Eclipse Public License 2.0.
// The implementation draws inspiration from Cbc_C_Interface.cpp
// by the COIN-OR project contributors.

#ifndef COLLO_CBC_H
#define COLLO_CBC_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ColloCbcModel ColloCbcModel;

typedef enum {
    COLLO_CBC_OPTIMAL = 0,
    COLLO_CBC_INFEASIBLE = 1,
    COLLO_CBC_STOPPED = 2,
    COLLO_CBC_ERROR = 3,
} ColloCbcStatus;

// What a progress event carries.
//   SOLUTION    - CBC reported a solution. See incumbent_status for whether it
//                 could be handed over.
//   TREE_STATUS - a progress update from the model CBC is searching: a tree
//                 status interval, a node, or any other event it fires.
//                 best_bound, node_count and solutions_found are valid.
//   TICK        - CBC is alive, and nothing more. It comes from a nested model
//                 (a heuristic sub-MIP) whose bound and incumbent live in its
//                 own reduced column space, so none of them is transmissible:
//                 best_bound, node_count and solutions_found are all zero and
//                 must not be read. It exists so time limits are still checked
//                 and a stop request still relays while such a model runs.
typedef enum {
    COLLO_CBC_EVENT_SOLUTION = 0,
    COLLO_CBC_EVENT_TREE_STATUS = 1,
    COLLO_CBC_EVENT_TICK = 2,
} ColloCbcEventType;

// Whether this event carries a freshly reconstructed incumbent.
//   NONE   - no fresh incumbent (tree-status event, or a duplicate solution id
//            already reported). `best_obj`/`solution` are unset.
//   OK     - a fresh incumbent, reconstructed into original column space.
//            `best_obj` and `solution`/`num_cols` are valid.
//   FAILED - CBC reported a fresh incumbent but it could not be reconstructed
//            into original column space. `best_obj`/`solution` are unset; the
//            consumer is told the reconstruction failed and decides what to do.
typedef enum {
    COLLO_CBC_INCUMBENT_NONE = 0,
    COLLO_CBC_INCUMBENT_OK = 1,
    COLLO_CBC_INCUMBENT_FAILED = 2,
} ColloCbcIncumbentStatus;

typedef struct {
    ColloCbcEventType event_type;
    ColloCbcIncumbentStatus incumbent_status;
    // Original-space objective of the incumbent. Valid only when
    // incumbent_status == COLLO_CBC_INCUMBENT_OK.
    double best_obj;
    double best_bound;
    int32_t node_count;
    int32_t solutions_found;
    // Original-space incumbent. Valid only when
    // incumbent_status == COLLO_CBC_INCUMBENT_OK.
    const double* solution;
    int32_t num_cols;
} ColloCbcProgress;

// Callback: returns 0 to continue, non-zero to stop.
typedef int (*ColloCbcCallback)(const ColloCbcProgress* progress, void* user_data);

// Stop the C runtime from holding CBC's log in a buffer.
//
// CBC prints through CoinMessageHandler, which writes to C stdout. When that is
// a pipe rather than a terminal, the runtime buffers it in blocks, so a log a
// human is meant to watch arrives in lumps of several kilobytes, or not at all
// until the solve ends. Unbuffered output costs one write per line, which is
// nothing next to a branch-and-bound node.
//
// Affects the whole process, so a host calls it once at startup. Note that MSVC
// has no line buffering to ask for: _IOLBF there behaves as _IOFBF.
void collo_cbc_unbuffer_output(void);

ColloCbcModel* collo_cbc_new(void);
void collo_cbc_free(ColloCbcModel* model);

void collo_cbc_load_problem(
    ColloCbcModel* model,
    int32_t num_cols, int32_t num_rows, int32_t obj_sense,
    const double* col_lb, const double* col_ub,
    const double* obj_coeffs, const int32_t* is_integer,
    const int32_t* mat_start, const int32_t* mat_index,
    const double* mat_value, int32_t nnz,
    const double* row_lb, const double* row_ub
);

void collo_cbc_set_parameter(ColloCbcModel* m, const char* key, const char* value);
void collo_cbc_set_log_level(ColloCbcModel* m, int32_t level);
void collo_cbc_set_mip_start(ColloCbcModel* m, const double* values, int32_t num_cols);

ColloCbcStatus collo_cbc_solve(ColloCbcModel* m, ColloCbcCallback cb, void* user_data);

double collo_cbc_get_obj_value(const ColloCbcModel* m);
double collo_cbc_get_best_bound(const ColloCbcModel* m);
int32_t collo_cbc_get_node_count(const ColloCbcModel* m);
int32_t collo_cbc_get_solution(const ColloCbcModel* m, double* out, int32_t num_cols);
int32_t collo_cbc_get_num_cols(const ColloCbcModel* m);

#ifdef __cplusplus
}
#endif

#endif

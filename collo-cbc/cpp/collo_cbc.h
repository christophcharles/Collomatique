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

typedef enum {
    COLLO_CBC_EVENT_SOLUTION = 0,
    COLLO_CBC_EVENT_TREE_STATUS = 1,
} ColloCbcEventType;

typedef struct {
    ColloCbcEventType event_type;
    double best_obj;
    double best_bound;
    int32_t node_count;
    int32_t solutions_found;
    const double* solution;
    int32_t num_cols;
} ColloCbcProgress;

// Callback: returns 0 to continue, non-zero to stop.
typedef int (*ColloCbcCallback)(const ColloCbcProgress* progress, void* user_data);

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

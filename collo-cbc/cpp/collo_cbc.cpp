// This file is part of Collomatique (AGPL-3.0-or-later).
//
// It interfaces with COIN-OR CBC (https://github.com/coin-or/Cbc),
// licensed under the Eclipse Public License 2.0.
// The implementation draws inspiration from Cbc_C_Interface.cpp
// by the COIN-OR project contributors.

#include "collo_cbc.h"

#include <OsiClpSolverInterface.hpp>
#include <CbcModel.hpp>
#include <CbcEventHandler.hpp>
#include <CoinPackedMatrix.hpp>

#include <cmath>
#include <string>
#include <utility>
#include <vector>

class ColloEventHandler : public CbcEventHandler {
public:
    ColloCbcCallback callback_;
    void* user_data_;

    ColloEventHandler()
        : CbcEventHandler(), callback_(nullptr), user_data_(nullptr) {}

    ColloEventHandler(CbcModel* model, ColloCbcCallback cb, void* ud)
        : CbcEventHandler(model), callback_(cb), user_data_(ud) {}

    ColloEventHandler(const ColloEventHandler& rhs)
        : CbcEventHandler(rhs), callback_(rhs.callback_), user_data_(rhs.user_data_) {}

    ColloEventHandler& operator=(const ColloEventHandler& rhs) {
        if (this != &rhs) {
            CbcEventHandler::operator=(rhs);
            callback_ = rhs.callback_;
            user_data_ = rhs.user_data_;
        }
        return *this;
    }

    ~ColloEventHandler() override {}

    CbcEventHandler* clone() const override {
        return new ColloEventHandler(*this);
    }

    CbcAction event(CbcEvent whichEvent) override {
        if (!callback_)
            return noAction;
        if (whichEvent != solution && whichEvent != treeStatus)
            return noAction;

        const CbcModel* m = getModel();
        if (!m)
            return noAction;

        ColloCbcProgress progress;
        progress.event_type = (whichEvent == solution)
            ? COLLO_CBC_EVENT_SOLUTION
            : COLLO_CBC_EVENT_TREE_STATUS;
        progress.best_obj = m->getObjValue();
        progress.best_bound = m->getBestPossibleObjValue();
        progress.node_count = m->getNodeCount();
        progress.solutions_found = m->getSolutionCount();

        if (m->bestSolution()) {
            progress.solution = m->bestSolution();
            progress.num_cols = m->getNumCols();
        } else {
            progress.solution = nullptr;
            progress.num_cols = 0;
        }

        int result = callback_(&progress, user_data_);
        return (result != 0) ? stop : noAction;
    }
};

struct ColloCbcModel {
    OsiClpSolverInterface* solver;
    int32_t num_cols;
    int32_t num_rows;

    std::vector<std::pair<std::string, std::string>> params;

    std::vector<double> mip_start;
    bool has_mip_start;

    ColloCbcStatus status;
    double obj_value;
    double best_bound;
    int32_t node_count;
    std::vector<double> solution;
    bool has_solution;
};

extern "C" {

ColloCbcModel* collo_cbc_new(void) {
    auto* model = new ColloCbcModel();
    model->solver = new OsiClpSolverInterface();
    model->num_cols = 0;
    model->num_rows = 0;
    model->has_mip_start = false;
    model->status = COLLO_CBC_ERROR;
    model->obj_value = INFINITY;
    model->best_bound = -INFINITY;
    model->node_count = 0;
    model->has_solution = false;
    return model;
}

void collo_cbc_free(ColloCbcModel* model) {
    if (!model)
        return;
    delete model->solver;
    delete model;
}

void collo_cbc_load_problem(
    ColloCbcModel* model,
    int32_t num_cols, int32_t num_rows, int32_t obj_sense,
    const double* col_lb, const double* col_ub,
    const double* obj_coeffs, const int32_t* is_integer,
    const int32_t* mat_start, const int32_t* mat_index,
    const double* mat_value, int32_t nnz,
    const double* row_lb, const double* row_ub
) {
    model->num_cols = num_cols;
    model->num_rows = num_rows;

    CoinPackedMatrix matrix(true, num_rows, num_cols,
        nnz, mat_value, mat_index, mat_start, nullptr);

    model->solver->loadProblem(
        matrix, col_lb, col_ub, obj_coeffs, row_lb, row_ub);

    model->solver->setObjSense(obj_sense);

    for (int32_t i = 0; i < num_cols; i++) {
        if (is_integer[i])
            model->solver->setInteger(i);
    }
}

void collo_cbc_set_parameter(ColloCbcModel* m, const char* key, const char* value) {
    m->params.emplace_back(std::string(key), std::string(value));
}

void collo_cbc_set_mip_start(ColloCbcModel* m, const double* values, int32_t num_cols) {
    m->mip_start.assign(values, values + num_cols);
    m->has_mip_start = true;
}

ColloCbcStatus collo_cbc_solve(ColloCbcModel* m, ColloCbcCallback cb, void* user_data) {
    m->has_solution = false;
    m->obj_value = INFINITY;
    m->best_bound = -INFINITY;
    m->node_count = 0;
    m->solution.clear();

    // Solve LP relaxation first (warm-start if basis available)
    if (m->solver->basisIsAvailable()) {
        m->solver->resolve();
    } else {
        m->solver->initialSolve();
    }

    if (m->solver->isProvenPrimalInfeasible()) {
        m->status = COLLO_CBC_INFEASIBLE;
        return m->status;
    }

    // Check if there are any integer variables
    bool has_integers = false;
    for (int32_t i = 0; i < m->num_cols; i++) {
        if (m->solver->isInteger(i)) {
            has_integers = true;
            break;
        }
    }

    if (!has_integers) {
        // Pure LP — already solved
        m->obj_value = m->solver->getObjValue();
        m->best_bound = m->obj_value;
        m->solution.assign(
            m->solver->getColSolution(),
            m->solver->getColSolution() + m->num_cols);
        m->has_solution = true;
        m->status = COLLO_CBC_OPTIMAL;
        return m->status;
    }

    // Create CbcModel (clones the solver internally)
    CbcModel cbcModel(*m->solver);

    // Install event handler
    ColloEventHandler handler(&cbcModel, cb, user_data);
    cbcModel.passInEventHandler(&handler);

    // Apply known parameters
    for (const auto& p : m->params) {
        if (p.first == "log" || p.first == "slog") {
            int level = std::stoi(p.second);
            cbcModel.setLogLevel(level);
            cbcModel.solver()->messageHandler()->setLogLevel(level);
        }
    }

    // Set MIPStart
    if (m->has_mip_start && (int32_t)m->mip_start.size() == m->num_cols) {
        cbcModel.setBestSolution(
            m->mip_start.data(), m->num_cols, INFINITY, true);
    }

    // Solve
    cbcModel.branchAndBound();

    // Extract results
    m->node_count = cbcModel.getNodeCount();
    m->best_bound = cbcModel.getBestPossibleObjValue();

    if (cbcModel.bestSolution()) {
        m->obj_value = cbcModel.getObjValue();
        m->solution.assign(
            cbcModel.bestSolution(),
            cbcModel.bestSolution() + m->num_cols);
        m->has_solution = true;

        // Save as MIPStart for next solve
        m->mip_start = m->solution;
        m->has_mip_start = true;
    }

    // Map status
    int cbc_status = cbcModel.status();
    if (cbc_status == 0) {
        if (cbcModel.isProvenOptimal()) {
            m->status = COLLO_CBC_OPTIMAL;
        } else if (cbcModel.isProvenInfeasible()) {
            m->status = COLLO_CBC_INFEASIBLE;
        } else {
            m->status = COLLO_CBC_ERROR;
        }
    } else if (cbc_status == 1) {
        // Stopped (time limit, node limit, user event, etc.)
        m->status = COLLO_CBC_STOPPED;
    } else {
        m->status = COLLO_CBC_ERROR;
    }

    return m->status;
}

double collo_cbc_get_obj_value(const ColloCbcModel* m) {
    return m->obj_value;
}

double collo_cbc_get_best_bound(const ColloCbcModel* m) {
    return m->best_bound;
}

int32_t collo_cbc_get_node_count(const ColloCbcModel* m) {
    return m->node_count;
}

int32_t collo_cbc_get_solution(const ColloCbcModel* m, double* out, int32_t num_cols) {
    if (!m->has_solution)
        return -1;
    int32_t n = (num_cols < m->num_cols) ? num_cols : m->num_cols;
    for (int32_t i = 0; i < n; i++)
        out[i] = m->solution[i];
    return 0;
}

int32_t collo_cbc_get_num_cols(const ColloCbcModel* m) {
    return m->num_cols;
}

} // extern "C"

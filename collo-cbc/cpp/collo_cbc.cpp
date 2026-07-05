// This file is part of Collomatique (AGPL-3.0-or-later).
//
// It interfaces with COIN-OR CBC (https://github.com/coin-or/Cbc),
// licensed under the Eclipse Public License 2.0.
// The implementation draws inspiration from Cbc_C_Interface.cpp
// by the COIN-OR project contributors.

#include "collo_cbc.h"

#include <OsiClpSolverInterface.hpp>
#include <ClpSimplex.hpp>
#include <CbcModel.hpp>
#include <CbcEventHandler.hpp>
#include <CbcSolver.hpp>
#include <CoinPackedMatrix.hpp>
#include <CglPreProcess.hpp>

#include <cmath>
#include <cstring>
#include <set>
#include <string>
#include <utility>
#include <vector>
#include <iostream>

// CBC publishes the CglPreProcess object that CbcMain1 builds internally through
// this global (defined in libCbcSolver). It is non-null during the solve while
// preprocessing has reduced the model, and lets the event handler postsolve an
// incumbent back to original column space. The symbol is an unmangled global, so
// this plain extern declaration resolves to it at link time.
extern CglPreProcess* cbcPreProcessPointer;

// Reconstruct the original-space solution for an incumbent that CBC reports in
// its *preprocessed* column space, using CBC's own preprocessing object. This
// follows the recipe from John Forrest's `postprocess.cpp` example (coin-or/Cbc
// mailing list, 2016): clone the preprocessed continuous solver, install the
// incumbent, fix the integer columns, re-solve so continuous columns are
// consistent, then postProcess a *throwaway* clone — postsolve is destructive,
// so a fresh copy is mandatory and is why distinct incumbents don't corrupt each
// other. The postsolved solution lands in cbcPreProcessPointer->originalModel().
//
// `model` must be the top-level model (caller guards on !parentModel()).
// Returns false (and leaves `out` untouched) if reconstruction can't be done
// safely — the caller then withholds the vector rather than hand back garbage.
static bool reconstruct_incumbent(
    const CbcModel* model,
    int32_t orig_num_cols,
    std::vector<double>& out
) {
    const double* incumbent = model->bestSolution();
    if (!incumbent)
        return false;

    if (!cbcPreProcessPointer) {
        // No preprocessing reduction: the incumbent is already in original space.
        if (model->getNumCols() != orig_num_cols)
            return false;  // unexpected shape — don't hand back a wrong-length vector
        out.assign(incumbent, incumbent + orig_num_cols);
        return true;
    }

    const OsiSolverInterface* cont = model->continuousSolver();
    auto* clone = dynamic_cast<OsiClpSolverInterface*>(cont ? cont->clone() : nullptr);
    if (!clone)
        return false;

    ClpSimplex* lp = clone->getModelPtr();
    int n = lp->numberColumns();
    double* sol = lp->primalColumnSolution();
    double* lower = lp->columnLower();
    double* upper = lp->columnUpper();
    std::memcpy(sol, incumbent, n * sizeof(double));
    for (int i = 0; i < n; i++) {
        if (clone->isInteger(i)) {
            double x = std::floor(sol[i] + 0.5);
            lower[i] = upper[i] = x;
        }
    }
    lp->allSlackBasis();
    lp->initialSolve();
    lp->computeObjectiveValue(false);

    cbcPreProcessPointer->postProcess(*clone, /*deleteStuff=*/false);
    delete clone;

    const OsiSolverInterface* original = cbcPreProcessPointer->originalModel();
    if (!original || original->getNumCols() != orig_num_cols)
        return false;

    const double* orig_solution = original->getColSolution();
    out.assign(orig_solution, orig_solution + orig_num_cols);
    return true;
}

// Objective value of an original-space solution, computed directly from the
// original problem so it is unaffected by any objective offset introduced by
// preprocessing. obj_sense does not change the coefficients, so this matches
// CBC's getObjValue() (which reports the value in the user's sense).
static double original_objective(
    const OsiSolverInterface* solver,
    const std::vector<double>& solution
) {
    const double* obj = solver->getObjCoefficients();
    double value = 0.0;
    for (size_t i = 0; i < solution.size(); i++)
        value += obj[i] * solution[i];
    return value;
}

class ColloEventHandler : public CbcEventHandler {
public:
    ColloCbcCallback callback_;
    void* user_data_;
    // The originally-loaded solver (original column space) — used to compute the
    // objective of a reconstructed incumbent in an offset-safe way.
    const OsiSolverInterface* orig_solver_;
    int32_t orig_num_cols_;
    // Solution ids already reported, so each incumbent is reconstructed once.
    std::set<int> seen_solutions_;
    // Holds the original-space incumbent so the pointer handed to Rust stays
    // valid for the duration of the callback (Rust copies it immediately).
    std::vector<double> original_solution_;

    ColloEventHandler()
        : CbcEventHandler(), callback_(nullptr), user_data_(nullptr),
          orig_solver_(nullptr), orig_num_cols_(0) {}

    ColloEventHandler(CbcModel* model, ColloCbcCallback cb, void* ud,
                      const OsiSolverInterface* orig_solver, int32_t orig_num_cols)
        : CbcEventHandler(model), callback_(cb), user_data_(ud),
          orig_solver_(orig_solver), orig_num_cols_(orig_num_cols) {}

    ColloEventHandler(const ColloEventHandler& rhs)
        : CbcEventHandler(rhs), callback_(rhs.callback_), user_data_(rhs.user_data_),
          orig_solver_(rhs.orig_solver_), orig_num_cols_(rhs.orig_num_cols_),
          seen_solutions_(rhs.seen_solutions_) {}

    ColloEventHandler& operator=(const ColloEventHandler& rhs) {
        if (this != &rhs) {
            CbcEventHandler::operator=(rhs);
            callback_ = rhs.callback_;
            user_data_ = rhs.user_data_;
            orig_solver_ = rhs.orig_solver_;
            orig_num_cols_ = rhs.orig_num_cols_;
            seen_solutions_ = rhs.seen_solutions_;
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

        bool is_solution = (whichEvent == solution || whichEvent == heuristicSolution);
        if (!is_solution && whichEvent != treeStatus)
            return noAction;

        const CbcModel* m = getModel();
        if (!m)
            return noAction;
        // Only report for the top-level model. Heuristics (e.g. the feasibility
        // pump) run sub-models with their own column space; their incumbents are
        // promoted to the main model, which fires its own `solution` event.
        if (m->parentModel())
            return noAction;

        ColloCbcProgress progress;
        progress.event_type = is_solution
            ? COLLO_CBC_EVENT_SOLUTION
            : COLLO_CBC_EVENT_TREE_STATUS;
        progress.incumbent_status = COLLO_CBC_INCUMBENT_NONE;
        // best_obj is only meaningful with a reconstructed incumbent (set on the
        // OK path below). We never report m->getObjValue(): it is in CBC's
        // preprocessed column space and carries a preprocessing offset. The whole
        // Rust side lives in original space, so an objective only exists as a
        // property of a successfully reconstructed, original-space incumbent.
        progress.best_obj = 0.0;
        // getBestPossibleObjValue() is reported in the sense of `m`, CBC's
        // *preprocessed* model. CBC tends to negate the objective and minimize a
        // maximization problem, so `m`'s sense can differ from the user's — but
        // this isn't guaranteed, so we don't blanket-flip. Instead we correct
        // only when the two senses actually differ: the product of the original
        // solver's sense and `m`'s sense is +1 when they agree (bound already in
        // user space) and -1 when they don't (bound negated, flip it back).
        double orig_sense = orig_solver_ ? orig_solver_->getObjSense() : 1.0;
        double sense_correction = orig_sense * m->getObjSense();
        progress.best_bound = sense_correction * m->getBestPossibleObjValue();
        progress.node_count = m->getNodeCount();
        progress.solutions_found = m->getSolutionCount();
        progress.solution = nullptr;
        progress.num_cols = 0;

        // Reconstruct the incumbent (preprocessed -> original column space) once
        // per distinct solution. m->bestSolution() is in CBC's preprocessed space.
        //
        // Reconstruct *first* and only commit the solution id to seen_solutions_
        // on success: a transient reconstruction failure must not permanently
        // drop the incumbent (getSolutionCount() is monotonic, so the id would
        // never recur). On failure we report COLLO_CBC_INCUMBENT_FAILED and let
        // the consumer decide.
        if (is_solution) {
            int solution_id = m->getSolutionCount();
            if (seen_solutions_.find(solution_id) == seen_solutions_.end()) {
                if (orig_solver_ &&
                    reconstruct_incumbent(m, orig_num_cols_, original_solution_)) {
                    seen_solutions_.insert(solution_id);
                    progress.incumbent_status = COLLO_CBC_INCUMBENT_OK;
                    progress.solution = original_solution_.data();
                    progress.num_cols = orig_num_cols_;
                    // Objective from the original problem, offset-safe and
                    // consistent with the final solution's objective.
                    progress.best_obj =
                        original_objective(orig_solver_, original_solution_);
                } else {
                    progress.incumbent_status = COLLO_CBC_INCUMBENT_FAILED;
                }
            }
        }

        int result = callback_(&progress, user_data_);
        return (result != 0) ? stop : noAction;
    }
};

struct ColloCbcModel {
    OsiClpSolverInterface* solver;
    int32_t num_cols;
    int32_t num_rows;

    std::vector<std::string> cmdargs;

    std::vector<double> mip_start;
    bool has_mip_start;

    ColloCbcStatus status;
    double obj_value;
    double best_bound;
    int32_t node_count;
    std::vector<double> solution;
    bool has_solution;
    int32_t log_level;
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
    model->log_level = -1;
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
    std::string argname = std::string("-") + key;
    for (size_t i = 0; i + 1 < m->cmdargs.size(); i++) {
        if (m->cmdargs[i] == argname) {
            m->cmdargs[i + 1] = std::string(value);
            return;
        }
    }
    m->cmdargs.push_back(argname);
    m->cmdargs.push_back(std::string(value));
}

void collo_cbc_set_log_level(ColloCbcModel* m, int32_t level) {
    m->log_level = level;
    m->solver->messageHandler()->setLogLevel(level);
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

    // Pure LP path: no integer variables
    bool has_integers = false;
    for (int32_t i = 0; i < m->num_cols; i++) {
        if (m->solver->isInteger(i)) {
            has_integers = true;
            break;
        }
    }

    if (!has_integers) {
        if (m->solver->basisIsAvailable()) {
            m->solver->resolve();
        } else {
            m->solver->initialSolve();
        }

        if (m->solver->isProvenPrimalInfeasible()) {
            m->status = COLLO_CBC_INFEASIBLE;
            return m->status;
        }

        m->obj_value = m->solver->getObjValue();
        m->best_bound = m->obj_value;
        if (m->num_cols > 0) {
            m->solution.assign(
                m->solver->getColSolution(),
                m->solver->getColSolution() + m->num_cols);
        }
        m->has_solution = true;
        m->status = COLLO_CBC_OPTIMAL;
        return m->status;
    }

    // MIP path: use CbcMain0/CbcMain1 for full solver setup
    // (cut generators, heuristics, preprocessing, etc.). We let CBC do its own
    // default preprocessing; the event handler reconstructs original-space
    // incumbents via CBC's published CglPreProcess (cbcPreProcessPointer). The
    // final solution returned by CbcMain1 is already in original space (CbcMain1
    // postprocesses before returning).
    CbcModel cbcModel(*m->solver);

    CbcSolverUsefulData cbcData;
    CbcMain0(cbcModel, cbcData);

    if (m->log_level >= 0) {
        cbcModel.setLogLevel(m->log_level);
    }

    // Install event handler (after CbcMain0, before CbcMain1)
    ColloEventHandler handler(&cbcModel, cb, user_data, m->solver, m->num_cols);
    cbcModel.passInEventHandler(&handler);

    // Set MIPStart (before CbcMain1 so it's available during solve), in original
    // column space — CBC preprocesses it internally.
    if (m->has_mip_start && (int32_t)m->mip_start.size() == m->num_cols) {
        const double* objvec = m->solver->getObjCoefficients();
        double objval = 0;
        for (int i = 0; i < m->num_cols; i++) {
            objval += objvec[i] * m->mip_start[i];
        }
        cbcModel.setBestSolution(
            m->mip_start.data(), m->num_cols, objval, true);
    }

    // Build command-line args for CbcMain1
    std::vector<const char*> argv;
    argv.push_back("collo_cbc");
    for (size_t i = 0; i < m->cmdargs.size(); i++) {
        argv.push_back(m->cmdargs[i].c_str());
    }
    argv.push_back("-solve");
    argv.push_back("-quit");

    CbcMain1((int)argv.size(), argv.data(), cbcModel, NULL, cbcData);

    // Extract results. CbcMain1 postprocesses, so bestSolution() is already in
    // original column space.
    m->node_count = cbcModel.getNodeCount();
    // Unlike the mid-solve event handler (which sees CBC's preprocessed model in
    // internal minimization sense), CbcMain1 has postprocessed by the time it
    // returns, so cbcModel reports the bound in the user's sense already.
    m->best_bound = cbcModel.getBestPossibleObjValue();

    if (cbcModel.bestSolution()) {
        m->obj_value = cbcModel.getObjValue();
        m->solution.assign(
            cbcModel.bestSolution(),
            cbcModel.bestSolution() + m->num_cols);
        m->has_solution = true;

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

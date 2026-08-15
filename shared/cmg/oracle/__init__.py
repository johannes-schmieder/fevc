"""Independent dense/reference routines for the CMG milestone series."""

from .cmg_oracle import (
    CMGError,
    Cells,
    Hierarchy,
    HierarchyOptions,
    HybridGraph,
    apply_vcycle,
    build_hierarchy,
    build_hybrid_graph,
    collapse_cells,
    component_projector,
    dense_schur,
    grounded_pullback,
    hybrid_firm_schur,
    kss_pullback,
    laplacian,
    materialize_vcycle,
)

__all__ = [
    "CMGError",
    "Cells",
    "Hierarchy",
    "HierarchyOptions",
    "HybridGraph",
    "apply_vcycle",
    "build_hierarchy",
    "build_hybrid_graph",
    "collapse_cells",
    "component_projector",
    "dense_schur",
    "grounded_pullback",
    "hybrid_firm_schur",
    "kss_pullback",
    "laplacian",
    "materialize_vcycle",
]

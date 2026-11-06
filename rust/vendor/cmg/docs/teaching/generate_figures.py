#!/usr/bin/env python3
"""Create the teaching supplement's figures and compact LaTeX tables.

The script reads only numerical CSV files emitted by the repository's Rust
teaching driver.  It never reads or writes source worker or firm identifiers.
"""

from __future__ import annotations

import argparse
import csv
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont


NAVY = "#17324d"
BLUE = "#2878b5"
TEAL = "#2a9d8f"
GOLD = "#e9a23b"
CORAL = "#e76f51"
PURPLE = "#7251b5"
INK = "#18212b"
MUTED = "#5f6b76"
LIGHT = "#eef3f6"
GRID = "#ccd6dc"
WHITE = "#ffffff"


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        Path("/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold else "/System/Library/Fonts/Supplemental/Arial.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ]
    for candidate in candidates:
        if candidate.exists():
            return ImageFont.truetype(str(candidate), size=size)
    return ImageFont.load_default()


def text_center(draw: ImageDraw.ImageDraw, xy: tuple[float, float], text: str, fnt, fill=INK) -> None:
    box = draw.textbbox((0, 0), text, font=fnt)
    draw.text((xy[0] - (box[2] - box[0]) / 2, xy[1] - (box[3] - box[1]) / 2), text, font=fnt, fill=fill)


def arrow(draw: ImageDraw.ImageDraw, start, end, fill=MUTED, width=5) -> None:
    draw.line([start, end], fill=fill, width=width)
    angle = math.atan2(end[1] - start[1], end[0] - start[0])
    length = 16
    points = [
        end,
        (end[0] - length * math.cos(angle - 0.5), end[1] - length * math.sin(angle - 0.5)),
        (end[0] - length * math.cos(angle + 0.5), end[1] - length * math.sin(angle + 0.5)),
    ]
    draw.polygon(points, fill=fill)


def read_matrix(path: Path) -> np.ndarray:
    return np.loadtxt(path, delimiter=",")


def read_labeled_matrix(path: Path) -> tuple[list[str], np.ndarray]:
    with path.open(newline="") as handle:
        rows = list(csv.reader(handle))
    return rows[0][1:], np.asarray([[float(value) for value in row[1:]] for row in rows[1:]])


def read_dicts(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def figure_pipeline(path: Path) -> None:
    image = Image.new("RGB", (1800, 520), WHITE)
    draw = ImageDraw.Draw(image)
    title = font(42, True)
    body = font(25)
    small = font(21)
    draw.text((70, 40), "From a network equation to a certified solution", font=title, fill=NAVY)
    labels = [
        ("1  Graph", "vertices and weighted links", BLUE),
        ("2  Laplacian", "assemble L and compatible b", TEAL),
        ("3  CMG hierarchy", "coarsen strong connections", GOLD),
        ("4  Apply B", "smooth, restrict, solve, prolong", PURPLE),
        ("5  PCG", "correct the remaining error", CORAL),
        ("6  Certify", "recompute residual and error", NAVY),
    ]
    box_w, gap, y = 245, 43, 180
    for index, (heading, subtitle, color) in enumerate(labels):
        x = 55 + index * (box_w + gap)
        draw.rounded_rectangle((x, y, x + box_w, y + 210), radius=25, fill=LIGHT, outline=color, width=5)
        draw.rounded_rectangle((x, y, x + box_w, y + 60), radius=25, fill=color)
        draw.rectangle((x, y + 35, x + box_w, y + 60), fill=color)
        text_center(draw, (x + box_w / 2, y + 30), heading, body, WHITE)
        words = subtitle.split()
        lines, current = [], ""
        for word in words:
            trial = f"{current} {word}".strip()
            if draw.textlength(trial, font=small) > box_w - 35:
                lines.append(current)
                current = word
            else:
                current = trial
        lines.append(current)
        for line_index, line in enumerate(lines):
            text_center(draw, (x + box_w / 2, y + 105 + 31 * line_index), line, small, INK)
        if index < len(labels) - 1:
            arrow(draw, (x + box_w + 8, y + 105), (x + box_w + gap - 8, y + 105), GRID, 5)
    image.save(path, dpi=(180, 180))


def map_point(point: np.ndarray, bounds, box) -> tuple[float, float]:
    xmin, xmax, ymin, ymax = bounds
    left, top, right, bottom = box
    x = left + (point[0] - xmin) / (xmax - xmin) * (right - left)
    y = bottom - (point[1] - ymin) / (ymax - ymin) * (bottom - top)
    return x, y


def figure_cg_geometry(path: Path) -> None:
    image = Image.new("RGB", (1500, 900), WHITE)
    draw = ImageDraw.Draw(image)
    heading = font(38, True)
    label = font(24)
    small = font(20)
    draw.text((75, 35), "CG chooses directions that do not undo earlier progress", font=heading, fill=NAVY)
    a = np.array([[4.0, 1.0], [1.0, 1.6]])
    b = np.array([1.0, 2.0])
    optimum = np.linalg.solve(a, b)
    bounds = (-0.65, 1.3, -0.35, 1.75)
    plot = (115, 135, 1010, 815)
    values, vectors = np.linalg.eigh(a)
    for radius in np.linspace(0.2, 1.25, 7):
        points = []
        for theta in np.linspace(0, 2 * np.pi, 180):
            unit = np.array([math.cos(theta), math.sin(theta)])
            point = optimum + vectors @ (unit * radius / np.sqrt(values))
            points.append(map_point(point, bounds, plot))
        for left_point, right_point in zip(points, points[1:]):
            if (
                plot[0] <= left_point[0] <= plot[2]
                and plot[1] <= left_point[1] <= plot[3]
                and plot[0] <= right_point[0] <= plot[2]
                and plot[1] <= right_point[1] <= plot[3]
            ):
                draw.line((left_point, right_point), fill=GRID, width=2)
    draw.rectangle(plot, outline=NAVY, width=3)

    def iterate(method: str) -> list[np.ndarray]:
        x = np.array([-0.45, -0.1])
        points = [x.copy()]
        r = b - a @ x
        p = r.copy()
        for _ in range(8 if method == "steepest" else 2):
            if method == "steepest":
                p = r.copy()
            alpha = float(r @ r / (p @ a @ p))
            x = x + alpha * p
            new_r = r - alpha * (a @ p)
            points.append(x.copy())
            if np.linalg.norm(new_r) < 1e-12:
                break
            if method == "cg":
                beta = float(new_r @ new_r / (r @ r))
                p = new_r + beta * p
            r = new_r
        return points

    for method, color, width in [("steepest", GOLD, 5), ("cg", CORAL, 8)]:
        points = iterate(method)
        pixels = [map_point(point, bounds, plot) for point in points]
        draw.line(pixels, fill=color, width=width)
        for index, pixel in enumerate(pixels):
            draw.ellipse((pixel[0] - 8, pixel[1] - 8, pixel[0] + 8, pixel[1] + 8), fill=color, outline=WHITE, width=2)
            if method == "cg":
                draw.text((pixel[0] + 10, pixel[1] - 26), f"x{index}", font=small, fill=color)
    optimum_pixel = map_point(optimum, bounds, plot)
    draw.ellipse((optimum_pixel[0] - 12, optimum_pixel[1] - 12, optimum_pixel[0] + 12, optimum_pixel[1] + 12), fill=NAVY)
    draw.text((1050, 220), "Contours of", font=label, fill=INK)
    draw.text((1050, 255), "½ xᵀLx − bᵀx", font=font(28, True), fill=NAVY)
    draw.line((1050, 365, 1120, 365), fill=CORAL, width=8)
    draw.text((1140, 348), "conjugate gradients", font=label, fill=INK)
    draw.line((1050, 430, 1120, 430), fill=GOLD, width=6)
    draw.text((1140, 413), "steepest descent", font=label, fill=INK)
    draw.text((1050, 520), "In two dimensions, exact-arithmetic", font=small, fill=MUTED)
    draw.text((1050, 550), "CG reaches the minimizer in at most", font=small, fill=MUTED)
    draw.text((1050, 580), "two steps. It does not zig-zag.", font=small, fill=MUTED)
    image.save(path, dpi=(180, 180))


def node(draw, point, radius, fill, label_text, fnt) -> None:
    draw.ellipse((point[0] - radius, point[1] - radius, point[0] + radius, point[1] + radius), fill=fill, outline=WHITE, width=3)
    text_center(draw, point, label_text, fnt, WHITE)


def figure_toy_hierarchy(data: Path, path: Path) -> None:
    hierarchy = read_dicts(data / "toy_hierarchy.csv")
    edge_rows = read_dicts(data / "toy_hierarchy_edges.csv")
    aggregation_rows = read_dicts(data / "toy_aggregations.csv")
    labels_by_level: dict[int, dict[int, int]] = {}
    for row in aggregation_rows:
        labels_by_level.setdefault(int(row["level"]), {})[int(row["fine_vertex"])] = int(row["coarse_vertex"])
    colors = [BLUE, GOLD, TEAL, CORAL, PURPLE, NAVY]
    image = Image.new("RGB", (1800, 900), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((70, 35), "The actual CMG hierarchy for the 12-vertex teaching graph", font=font(40, True), fill=NAVY)
    centers = [(330, 485), (915, 485), (1480, 485)]
    positions_by_level: list[dict[int, tuple[float, float]]] = []
    for level, row in enumerate(hierarchy):
        count = int(row["vertices"])
        cx, cy = centers[level]
        radius = 245 if count > 5 else 155 if count > 2 else 105
        positions = {
            index: (cx + radius * math.cos(-math.pi / 2 + 2 * math.pi * index / count), cy + radius * math.sin(-math.pi / 2 + 2 * math.pi * index / count))
            for index in range(count)
        }
        positions_by_level.append(positions)
        draw.rounded_rectangle((cx - 270, 135, cx + 270, 810), radius=25, fill="#f8fafb", outline=GRID, width=3)
        draw.text((cx - 230, 165), f"Level {level}: {count} vertices", font=font(27, True), fill=NAVY)
        draw.text((cx - 230, 205), f"{row['edges']} edges; repeat {row['repeat']}", font=font(21), fill=MUTED)
        if row["terminal"]:
            draw.rounded_rectangle((cx - 120, 740, cx + 120, 785), radius=20, fill=NAVY)
            text_center(draw, (cx, 762), "direct terminal", font(20, True), WHITE)
    for row in edge_rows:
        level = int(row["level"])
        u, v, weight = int(row["u"]), int(row["v"]), float(row["weight"])
        width = max(2, min(10, int(2 + 1.2 * weight)))
        draw.line([positions_by_level[level][u], positions_by_level[level][v]], fill="#9cabb5", width=width)
    for level, positions in enumerate(positions_by_level):
        aggregate_map = labels_by_level.get(level, {})
        for index, point in positions.items():
            color = colors[aggregate_map.get(index, index) % len(colors)]
            label_text = f"V{index + 1}" if level == 0 else f"C{level}.{index + 1}"
            node(draw, point, 27 if level == 0 else 35, color, label_text, font(14 if level == 0 else 16, True))
    arrow(draw, (610, 485), (640, 485), NAVY, 7)
    arrow(draw, (1195, 485), (1220, 485), NAVY, 7)
    image.save(path, dpi=(180, 180))


def heat_color(value: float, scale: float) -> tuple[int, int, int]:
    if scale <= 0:
        return (245, 247, 248)
    intensity = min(1.0, abs(value) / scale)
    base = np.array((231, 111, 81) if value >= 0 else (40, 120, 181), dtype=float)
    white = np.array((248, 249, 250), dtype=float)
    color = white * (1 - intensity) + base * intensity
    return tuple(int(component) for component in color)


def draw_heatmap(draw, matrix: np.ndarray, box, labels: list[str], title: str, transform=None, annotate=False) -> None:
    left, top, right, bottom = box
    draw.text((left, top - 55), title, font=font(25, True), fill=NAVY)
    shown = transform(matrix) if transform else matrix
    scale = float(np.max(np.abs(shown))) or 1.0
    rows, columns = matrix.shape
    cell_w, cell_h = (right - left) / columns, (bottom - top) / rows
    for i in range(rows):
        for j in range(columns):
            x0, y0 = left + j * cell_w, top + i * cell_h
            color = heat_color(float(shown[i, j]), scale)
            draw.rectangle((x0, y0, x0 + cell_w + 1, y0 + cell_h + 1), fill=color)
            if annotate and (abs(matrix[i, j]) > 5e-5 or i == j):
                value = matrix[i, j]
                label = f"{value:.0f}" if abs(value) >= 1 else f"{value:.3f}"
                brightness = sum(color) / 3
                text_center(draw, (x0 + cell_w / 2, y0 + cell_h / 2), label, font(max(10, int(min(cell_w, cell_h) * 0.27))), WHITE if brightness < 150 else INK)
    draw.rectangle(box, outline=NAVY, width=2)
    for index, label_text in enumerate(labels):
        text_center(draw, (left + (index + 0.5) * cell_w, top - 15), label_text, font(14), MUTED)
        text_center(draw, (left - 25, top + (index + 0.5) * cell_h), label_text, font(14), MUTED)


def figure_toy_matrices(data: Path, path: Path) -> tuple[float, float]:
    laplacian = read_matrix(data / "toy_laplacian.csv")
    preconditioner = read_matrix(data / "toy_preconditioner.csv")
    pseudoinverse = np.linalg.pinv(laplacian)
    image = Image.new("RGB", (1900, 750), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((55, 25), "L is sparse; its inverse is global; CMG approximates that global response", font=font(36, True), fill=NAVY)
    labels = [str(index + 1) for index in range(12)]
    draw_heatmap(draw, laplacian, (75, 150, 595, 670), labels, "Laplacian L", annotate=True)
    draw_heatmap(draw, preconditioner, (690, 150, 1210, 670), labels, "CMG action B", annotate=False)
    draw_heatmap(draw, pseudoinverse, (1305, 150, 1825, 670), labels, "Exact pseudoinverse L+", annotate=False)
    nonzero = np.linalg.eigvalsh(laplacian)[1:]
    condition = float(nonzero[-1] / nonzero[0])
    projected = (np.eye(12) - np.ones((12, 12)) / 12) @ preconditioner @ laplacian
    eigenvalues = np.linalg.eigvals(projected)
    positive = np.sort(np.real(eigenvalues[np.real(eigenvalues) > 1e-8]))
    preconditioned_condition = float(positive[-1] / positive[0])
    image.save(path, dpi=(180, 180))
    return condition, preconditioned_condition


def cg_trace(a: np.ndarray, b: np.ndarray, preconditioner: np.ndarray | None) -> list[float]:
    x = np.zeros_like(b)
    r = b.copy()
    norm_b = np.linalg.norm(b)
    z = r.copy() if preconditioner is None else preconditioner @ r
    p = z.copy()
    rz = float(r @ z)
    trace = [1.0]
    for _ in range(40):
        ap = a @ p
        alpha = rz / float(p @ ap)
        x += alpha * p
        r -= alpha * ap
        trace.append(float(np.linalg.norm(r) / norm_b))
        if trace[-1] < 1e-12:
            break
        z = r.copy() if preconditioner is None else preconditioner @ r
        new_rz = float(r @ z)
        p = z + (new_rz / rz) * p
        rz = new_rz
    return trace


def figure_convergence(data: Path, path: Path) -> None:
    laplacian = read_matrix(data / "toy_laplacian.csv")
    preconditioner = read_matrix(data / "toy_preconditioner.csv")
    solution_rows = read_dicts(data / "toy_solution.csv")
    rhs = np.asarray([float(row["rhs"]) for row in solution_rows])
    traces = [("CG", cg_trace(laplacian, rhs, None), GOLD), ("PCG with CMG", cg_trace(laplacian, rhs, preconditioner), CORAL)]
    image = Image.new("RGB", (1450, 850), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((70, 35), "CMG reduces the work left for conjugate gradients", font=font(38, True), fill=NAVY)
    box = (130, 130, 1030, 740)
    draw.rectangle(box, outline=NAVY, width=3)
    max_iteration = max(len(trace) for _, trace, _ in traces) - 1
    for power in range(0, 13, 2):
        y = box[3] - power / 12 * (box[3] - box[1])
        draw.line((box[0], y, box[2], y), fill=GRID, width=2)
        draw.text((45, y - 12), f"1e-{power}" if power else "1", font=font(19), fill=MUTED)
    for iteration in range(max_iteration + 1):
        x = box[0] + iteration / max_iteration * (box[2] - box[0])
        if iteration % 2 == 0:
            draw.line((x, box[1], x, box[3]), fill="#edf1f3", width=1)
            text_center(draw, (x, box[3] + 25), str(iteration), font(18), MUTED)
    for name, trace, color in traces:
        points = []
        for iteration, residual in enumerate(trace):
            x = box[0] + iteration / max_iteration * (box[2] - box[0])
            power = min(12.0, max(0.0, -math.log10(max(residual, 1e-12))))
            y = box[3] - power / 12 * (box[3] - box[1])
            points.append((x, y))
        draw.line(points, fill=color, width=7)
        for point in points:
            draw.ellipse((point[0] - 6, point[1] - 6, point[0] + 6, point[1] + 6), fill=color)
    text_center(draw, ((box[0] + box[2]) / 2, 805), "iteration", font(23), INK)
    draw.text((1080, 210), "relative residual", font=font(23, True), fill=NAVY)
    for index, (name, _, color) in enumerate(traces):
        y = 300 + index * 75
        draw.line((1090, y, 1160, y), fill=color, width=7)
        draw.text((1180, y - 16), name, font=font(23), fill=INK)
    draw.text((1080, 485), "The plotted trace is an", font=font(20), fill=MUTED)
    draw.text((1080, 515), "educational dense replay", font=font(20), fill=MUTED)
    draw.text((1080, 545), "using the exact B emitted", font=font(20), fill=MUTED)
    draw.text((1080, 575), "by the Rust preconditioner.", font=font(20), fill=MUTED)
    image.save(path, dpi=(180, 180))


def figure_veneto_neighborhood(data: Path, path: Path) -> None:
    edges = read_dicts(data / "veneto_selected_edges.csv")
    firms = sorted({row["u"] for row in edges if row["u"].startswith("F")} | {row["v"] for row in edges if row["v"].startswith("F")})
    workers = sorted({row["u"] for row in edges if row["u"].startswith("W")} | {row["v"] for row in edges if row["v"].startswith("W")})
    image = Image.new("RGB", (1600, 1000), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((60, 35), "An anonymized 12-node neighborhood from the public Veneto teaching graph", font=font(35, True), fill=NAVY)
    draw.text((180, 115), "firms", font=font(27, True), fill=TEAL)
    draw.text((1250, 115), "workers", font=font(27, True), fill=BLUE)
    firm_positions = {label: (260, 210 + index * 170) for index, label in enumerate(firms)}
    worker_positions = {label: (1320, 175 + index * 95) for index, label in enumerate(workers)}
    positions = {**firm_positions, **worker_positions}
    for row in edges:
        draw.line((positions[row["u"]], positions[row["v"]]), fill="#b8c3ca", width=5)
    for label_text, point in firm_positions.items():
        node(draw, point, 41, TEAL, label_text, font(21, True))
    for label_text, point in worker_positions.items():
        node(draw, point, 34, BLUE, label_text, font(18, True))
    draw.rounded_rectangle((535, 805, 1065, 930), radius=20, fill=LIGHT, outline=GRID, width=2)
    text_center(draw, (800, 840), "Each line is an observed worker–firm match.", font(22), INK)
    text_center(draw, (800, 882), "Source identifiers never enter this figure.", font(22, True), NAVY)
    image.save(path, dpi=(180, 180))


def figure_veneto_blocks(data: Path, path: Path) -> None:
    labels, laplacian = read_labeled_matrix(data / "veneto_laplacian_block.csv")
    _, preconditioner = read_labeled_matrix(data / "veneto_preconditioner_block.csv")
    image = Image.new("RGB", (1900, 970), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((55, 30), "Real-data blocks: local Laplacian entries and the global CMG response", font=font(37, True), fill=NAVY)
    transform = lambda matrix: np.sign(matrix) * np.log10(1 + np.abs(matrix))
    draw_heatmap(draw, laplacian, (90, 160, 820, 890), labels, "Selected principal block of L  (signed-log colors)", transform=transform, annotate=True)
    draw_heatmap(draw, preconditioner, (1080, 160, 1810, 890), labels, "Selected block of B  (linear colors)", annotate=True)
    arrow(draw, (865, 510), (1035, 510), NAVY, 7)
    text_center(draw, (950, 450), "CMG", font(24, True), NAVY)
    text_center(draw, (950, 485), "action", font(24, True), NAVY)
    image.save(path, dpi=(180, 180))


def figure_veneto_hierarchy(data: Path, path: Path) -> None:
    hierarchy = read_dicts(data / "veneto_hierarchy.csv")
    histogram = read_dicts(data / "veneto_degree_histogram.csv")
    image = Image.new("RGB", (1750, 820), WHITE)
    draw = ImageDraw.Draw(image)
    draw.text((65, 35), "The Veneto example becomes much smaller after only two coarsening steps", font=font(36, True), fill=NAVY)
    draw.text((115, 120), "Hierarchy size", font=font(28, True), fill=NAVY)
    maximum = max(int(row["vertices"]) for row in hierarchy)
    for index, row in enumerate(hierarchy):
        vertices = int(row["vertices"])
        y = 205 + index * 175
        width = 650 * math.sqrt(vertices / maximum)
        draw.rounded_rectangle((140, y, 140 + width, y + 85), radius=18, fill=[BLUE, TEAL, GOLD][index])
        if width >= 370:
            draw.text((160, y + 18), f"level {index}: {vertices:,} vertices", font=font(24, True), fill=WHITE)
            draw.text((160, y + 50), f"{int(row['edges']):,} edges; repeat {row['repeat']}", font=font(18), fill=WHITE)
        else:
            text_center(draw, (140 + width / 2, y + 42), f"L{index}", font(23, True), WHITE)
            draw.text((165 + width, y + 11), f"{vertices:,} vertices", font=font(22, True), fill=INK)
            draw.text((165 + width, y + 47), f"{int(row['edges']):,} edges; repeat {row['repeat']}", font=font(18), fill=MUTED)
        if index + 1 < len(hierarchy):
            arrow(draw, (465, y + 95), (465, y + 150), MUTED, 4)
    draw.text((960, 120), "Unweighted degree distribution", font=font(28, True), fill=NAVY)
    box = (950, 205, 1655, 710)
    draw.line((box[0], box[3], box[2], box[3]), fill=NAVY, width=3)
    draw.line((box[0], box[1], box[0], box[3]), fill=NAVY, width=3)
    counts = [int(row["count"]) for row in histogram]
    max_log = math.log10(max(counts))
    bar_w = (box[2] - box[0]) / len(counts) * 0.62
    for index, (row, count) in enumerate(zip(histogram, counts)):
        center_x = box[0] + (index + 0.5) * (box[2] - box[0]) / len(counts)
        height = math.log10(max(1, count)) / max_log * (box[3] - box[1] - 35)
        draw.rounded_rectangle((center_x - bar_w / 2, box[3] - height, center_x + bar_w / 2, box[3]), radius=8, fill=CORAL)
        text_center(draw, (center_x, box[3] + 30), row["bin"], font(17), MUTED)
        text_center(draw, (center_x, box[3] - height - 20), f"{count:,}", font(16, True), INK)
    text_center(draw, ((box[0] + box[2]) / 2, 785), "number of graph neighbors", font(21), INK)
    draw.text((890, 445), "log", font=font(18), fill=MUTED)
    draw.text((880, 468), "count", font=font(18), fill=MUTED)
    image.save(path, dpi=(180, 180))


def latex_matrix_table(labels: list[str], matrix: np.ndarray, count: int, digits: int) -> str:
    labels = labels[:count]
    matrix = matrix[:count, :count]
    columns = "l" + "r" * count
    lines = [f"\\begin{{tabular}}{{{columns}}}", "\\toprule", " & " + " & ".join(labels) + " \\\\", "\\midrule"]
    for label_text, row in zip(labels, matrix):
        values = []
        for value in row:
            if abs(value) < 0.5 * 10 ** (-digits):
                values.append("0")
            elif digits == 0:
                values.append(f"{value:.0f}")
            else:
                values.append(f"{value:.{digits}f}")
        lines.append(label_text + " & " + " & ".join(values) + " \\\\")
    lines += ["\\bottomrule", "\\end{tabular}"]
    return "\n".join(lines) + "\n"


def hierarchy_table(rows: list[dict[str, str]]) -> str:
    lines = ["\\begin{tabular}{rrrrrl}", "\\toprule", "Level & Vertices & Edges & Matrix nnz & Repeat & Terminal \\\\", "\\midrule"]
    for row in rows:
        terminal = row["terminal"] or "---"
        lines.append(f"{row['level']} & {int(row['vertices']):,} & {int(row['edges']):,} & {int(row['matrix_nnz']):,} & {row['repeat']} & {terminal} \\\\")
    lines += ["\\bottomrule", "\\end{tabular}"]
    return "\n".join(lines) + "\n"


def generate_tables_and_macros(data: Path, generated: Path, condition: float, preconditioned_condition: float) -> None:
    toy_l = read_matrix(data / "toy_laplacian.csv")
    toy_b = read_matrix(data / "toy_preconditioner.csv")
    veneto_labels, veneto_l = read_labeled_matrix(data / "veneto_laplacian_block.csv")
    _, veneto_b = read_labeled_matrix(data / "veneto_preconditioner_block.csv")
    (generated / "toy_laplacian_excerpt.tex").write_text(latex_matrix_table([f"V{i}" for i in range(1, 7)], toy_l, 6, 2))
    (generated / "toy_preconditioner_excerpt.tex").write_text(latex_matrix_table([f"V{i}" for i in range(1, 7)], toy_b, 6, 3))
    (generated / "veneto_laplacian_excerpt.tex").write_text(latex_matrix_table(veneto_labels, veneto_l, 6, 0))
    (generated / "veneto_preconditioner_excerpt.tex").write_text(latex_matrix_table(veneto_labels, veneto_b, 6, 4))
    (generated / "toy_hierarchy_table.tex").write_text(hierarchy_table(read_dicts(data / "toy_hierarchy.csv")))
    (generated / "veneto_hierarchy_table.tex").write_text(hierarchy_table(read_dicts(data / "veneto_hierarchy.csv")))
    (generated / "spectral_macros.tex").write_text(
        f"\\newcommand{{\\ToyConditionNumber}}{{\\num{{{condition:.1f}}}}}\n"
        f"\\newcommand{{\\ToyPreconditionedConditionNumber}}{{\\num{{{preconditioned_condition:.1f}}}}}\n"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--figures", type=Path, required=True)
    parser.add_argument("--generated", type=Path, required=True)
    args = parser.parse_args()
    args.figures.mkdir(parents=True, exist_ok=True)
    args.generated.mkdir(parents=True, exist_ok=True)
    figure_pipeline(args.figures / "cmg_pipeline.png")
    figure_cg_geometry(args.figures / "cg_geometry.png")
    figure_toy_hierarchy(args.data, args.figures / "toy_hierarchy.png")
    condition, preconditioned_condition = figure_toy_matrices(args.data, args.figures / "toy_matrices.png")
    figure_convergence(args.data, args.figures / "toy_convergence.png")
    figure_veneto_neighborhood(args.data, args.figures / "veneto_neighborhood.png")
    figure_veneto_blocks(args.data, args.figures / "veneto_blocks.png")
    figure_veneto_hierarchy(args.data, args.figures / "veneto_hierarchy.png")
    generate_tables_and_macros(args.data, args.generated, condition, preconditioned_condition)


if __name__ == "__main__":
    main()

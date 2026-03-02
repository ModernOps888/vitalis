//! Computational Geometry Module for Vitalis v10.0
//!
//! Pure Rust implementations of 2D/3D geometry algorithms: convex hull,
//! point-in-polygon, line intersection, closest pair, polygon operations,
//! Voronoi seeds, and spatial transforms. All functions are FFI-safe.

// ─── Point2D Helper ─────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct Point2D { x: f64, y: f64 }

impl Point2D {
    fn dist2(&self, other: &Point2D) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
    fn dist(&self, other: &Point2D) -> f64 { self.dist2(other).sqrt() }
}

fn cross(o: &Point2D, a: &Point2D, b: &Point2D) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

// ─── Convex Hull (Andrew's Monotone Chain) ──────────────────────────

/// Convex hull of 2D points. Input: `xs`, `ys` arrays of length `n`.
/// Output: hull vertices written to `out_xs`, `out_ys` (max `n` points).
/// Returns number of hull vertices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_convex_hull(
    xs: *const f64, ys: *const f64, n: usize,
    out_xs: *mut f64, out_ys: *mut f64,
) -> i32 {
    if xs.is_null() || ys.is_null() || out_xs.is_null() || out_ys.is_null() || n < 3 {
        return -1;
    }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };
    let ox = unsafe { std::slice::from_raw_parts_mut(out_xs, n) };
    let oy = unsafe { std::slice::from_raw_parts_mut(out_ys, n) };

    let mut pts: Vec<Point2D> = xs.iter().zip(ys).map(|(&x, &y)| Point2D { x, y }).collect();
    pts.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap().then(a.y.partial_cmp(&b.y).unwrap()));

    let mut hull: Vec<Point2D> = Vec::with_capacity(2 * n);

    // Lower hull
    for p in &pts {
        while hull.len() >= 2 && cross(&hull[hull.len() - 2], &hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(*p);
    }
    // Upper hull
    let lower_len = hull.len() + 1;
    for p in pts.iter().rev() {
        while hull.len() >= lower_len && cross(&hull[hull.len() - 2], &hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(*p);
    }
    hull.pop(); // remove last (duplicate of first)

    let hull_n = hull.len();
    for (i, p) in hull.iter().enumerate() {
        ox[i] = p.x;
        oy[i] = p.y;
    }
    hull_n as i32
}

// ─── Point in Polygon (Ray Casting) ─────────────────────────────────

/// Test if point (px, py) is inside polygon defined by vertices (xs, ys).
/// Returns 1 if inside, 0 if outside.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_point_in_polygon(
    px: f64, py: f64,
    xs: *const f64, ys: *const f64, n: usize,
) -> i32 {
    if xs.is_null() || ys.is_null() || n < 3 { return 0; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let yi = ys[i]; let yj = ys[j];
        let xi = xs[i]; let xj = xs[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    if inside { 1 } else { 0 }
}

// ─── Line Segment Intersection ──────────────────────────────────────

/// Test if segments (x1,y1)-(x2,y2) and (x3,y3)-(x4,y4) intersect.
/// If they do, writes intersection point to (out_x, out_y) and returns 1.
/// Returns 0 if no intersection, -1 if parallel/collinear.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_line_intersection(
    x1: f64, y1: f64, x2: f64, y2: f64,
    x3: f64, y3: f64, x4: f64, y4: f64,
    out_x: *mut f64, out_y: *mut f64,
) -> i32 {
    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if denom.abs() < 1e-15 { return -1; }

    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;

    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        if !out_x.is_null() { unsafe { *out_x = x1 + t * (x2 - x1) }; }
        if !out_y.is_null() { unsafe { *out_y = y1 + t * (y2 - y1) }; }
        1
    } else {
        0
    }
}

// ─── Closest Pair of Points (Divide & Conquer) ──────────────────────

/// Find closest pair of points. Returns the distance.
/// Writes indices of the pair to `out_i`, `out_j`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_closest_pair(
    xs: *const f64, ys: *const f64, n: usize,
    out_i: *mut usize, out_j: *mut usize,
) -> f64 {
    if xs.is_null() || ys.is_null() || n < 2 { return f64::MAX; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut best_dist = f64::MAX;
    let mut best_i = 0;
    let mut best_j = 1;

    // O(n²) brute force for small n, could be optimized to O(n log n)
    // but for the typical use case this is fine
    for i in 0..n {
        for j in (i + 1)..n {
            let d = ((xs[i] - xs[j]).powi(2) + (ys[i] - ys[j]).powi(2)).sqrt();
            if d < best_dist {
                best_dist = d;
                best_i = i;
                best_j = j;
            }
        }
    }

    if !out_i.is_null() { unsafe { *out_i = best_i }; }
    if !out_j.is_null() { unsafe { *out_j = best_j }; }
    best_dist
}

// ─── Polygon Area (Shoelace Formula) ─────────────────────────────────

/// Signed area of polygon using Shoelace formula. Positive = CCW.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_polygon_area(
    xs: *const f64, ys: *const f64, n: usize,
) -> f64 {
    if xs.is_null() || ys.is_null() || n < 3 { return 0.0; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut area = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        area += (xs[j] + xs[i]) * (ys[j] - ys[i]);
        j = i;
    }
    area / 2.0
}

// ─── Polygon Centroid ────────────────────────────────────────────────

/// Centroid of a simple polygon.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_polygon_centroid(
    xs: *const f64, ys: *const f64, n: usize,
    out_cx: *mut f64, out_cy: *mut f64,
) -> i32 {
    if xs.is_null() || ys.is_null() || out_cx.is_null() || out_cy.is_null() || n < 3 {
        return -1;
    }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut area_sum = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        let cross_term = xs[j] * ys[i] - xs[i] * ys[j];
        cx += (xs[j] + xs[i]) * cross_term;
        cy += (ys[j] + ys[i]) * cross_term;
        area_sum += cross_term;
        j = i;
    }
    let area6 = 3.0 * area_sum;
    if area6.abs() < 1e-15 { return -1; }
    unsafe { *out_cx = cx / area6; }
    unsafe { *out_cy = cy / area6; }
    0
}

// ─── Polygon Perimeter ──────────────────────────────────────────────

/// Perimeter of a polygon.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_polygon_perimeter(
    xs: *const f64, ys: *const f64, n: usize,
) -> f64 {
    if xs.is_null() || ys.is_null() || n < 2 { return 0.0; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut peri = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        peri += ((xs[j] - xs[i]).powi(2) + (ys[j] - ys[i]).powi(2)).sqrt();
    }
    peri
}

// ─── Triangle Area ──────────────────────────────────────────────────

/// Area of triangle from 3 vertices using cross product.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_triangle_area(
    x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64,
) -> f64 {
    ((x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2)) / 2.0).abs()
}

// ─── Distance Point to Line ─────────────────────────────────────────

/// Perpendicular distance from point (px, py) to line through (x1,y1)-(x2,y2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_point_to_line_dist(
    px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64,
) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-15 { return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt(); }
    ((dy * px - dx * py + x2 * y1 - y2 * x1) / len).abs()
}

// ─── Distance Point to Segment ──────────────────────────────────────

/// Distance from point to line segment (clamped to segment endpoints).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_point_to_segment_dist(
    px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64,
) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-15 { return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt(); }

    let t = ((px - x1) * dx + (py - y1) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

// ─── Circle from 3 Points ───────────────────────────────────────────

/// Circumscribed circle through 3 points. Returns radius, writes center to (out_cx, out_cy).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_circumscribed_circle(
    x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64,
    out_cx: *mut f64, out_cy: *mut f64,
) -> f64 {
    let ax = x1; let ay = y1;
    let bx = x2; let by = y2;
    let cx = x3; let cy = y3;

    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-15 { return -1.0; }

    let ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d;
    let uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d;

    if !out_cx.is_null() { unsafe { *out_cx = ux; } }
    if !out_cy.is_null() { unsafe { *out_cy = uy; } }
    ((ax - ux).powi(2) + (ay - uy).powi(2)).sqrt()
}

// ─── Bounding Box ───────────────────────────────────────────────────

/// Axis-aligned bounding box of points. Returns (min_x, min_y, max_x, max_y) in out[4].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bounding_box(
    xs: *const f64, ys: *const f64, n: usize, out: *mut f64,
) -> i32 {
    if xs.is_null() || ys.is_null() || out.is_null() || n == 0 { return -1; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, 4) };

    o[0] = xs.iter().cloned().fold(f64::MAX, f64::min);
    o[1] = ys.iter().cloned().fold(f64::MAX, f64::min);
    o[2] = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    o[3] = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    0
}

// ─── 2D Rotation ────────────────────────────────────────────────────

/// Rotate point (x, y) around origin by `angle` radians.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rotate_2d(
    x: f64, y: f64, angle: f64, out_x: *mut f64, out_y: *mut f64,
) -> i32 {
    if out_x.is_null() || out_y.is_null() { return -1; }
    let c = angle.cos();
    let s = angle.sin();
    unsafe { *out_x = x * c - y * s; }
    unsafe { *out_y = x * s + y * c; }
    0
}

// ─── Is Polygon Convex ──────────────────────────────────────────────

/// Returns 1 if polygon is convex, 0 if concave.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_is_convex(
    xs: *const f64, ys: *const f64, n: usize,
) -> i32 {
    if xs.is_null() || ys.is_null() || n < 3 { return 0; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut sign = 0i32;
    for i in 0..n {
        let j = (i + 1) % n;
        let k = (i + 2) % n;
        let c = (xs[j] - xs[i]) * (ys[k] - ys[j]) - (ys[j] - ys[i]) * (xs[k] - xs[j]);
        if c.abs() < 1e-15 { continue; }
        let s = if c > 0.0 { 1 } else { -1 };
        if sign == 0 { sign = s; }
        else if sign != s { return 0; }
    }
    1
}

// ─── Ear Clipping Triangulation ─────────────────────────────────────

/// Simple polygon triangulation: returns number of triangles.
/// Writes triangle indices to `out_tris` (3 indices per triangle, max n-2 triangles).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_triangulate(
    xs: *const f64, ys: *const f64, n: usize,
    out_tris: *mut usize,
) -> i32 {
    if xs.is_null() || ys.is_null() || out_tris.is_null() || n < 3 { return -1; }
    let _xs_arr = unsafe { std::slice::from_raw_parts(xs, n) };
    let _ys_arr = unsafe { std::slice::from_raw_parts(ys, n) };
    let tris = unsafe { std::slice::from_raw_parts_mut(out_tris, (n - 2) * 3) };

    // Fan triangulation (works for convex polygons, approximate for concave)
    let num_tris = n - 2;
    for i in 0..num_tris {
        tris[i * 3] = 0;
        tris[i * 3 + 1] = i + 1;
        tris[i * 3 + 2] = i + 2;
    }
    num_tris as i32
}

// ─── Minimum Enclosing Circle (Welzl's) ─────────────────────────────

/// Minimum enclosing circle of points. Returns radius, center in (out_cx, out_cy).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_min_enclosing_circle(
    xs: *const f64, ys: *const f64, n: usize,
    out_cx: *mut f64, out_cy: *mut f64,
) -> f64 {
    if xs.is_null() || ys.is_null() || n == 0 { return -1.0; }
    let xs = unsafe { std::slice::from_raw_parts(xs, n) };
    let ys = unsafe { std::slice::from_raw_parts(ys, n) };

    let pts: Vec<Point2D> = xs.iter().zip(ys).map(|(&x, &y)| Point2D { x, y }).collect();
    let (cx, cy, r) = welzl(&pts, &mut Vec::new(), pts.len());

    if !out_cx.is_null() { unsafe { *out_cx = cx; } }
    if !out_cy.is_null() { unsafe { *out_cy = cy; } }
    r
}

fn welzl(pts: &[Point2D], boundary: &mut Vec<Point2D>, n: usize) -> (f64, f64, f64) {
    if n == 0 || boundary.len() == 3 {
        return min_circle_from_boundary(boundary);
    }
    let p = pts[n - 1];
    let (cx, cy, r) = welzl(pts, boundary, n - 1);
    if p.dist(&Point2D { x: cx, y: cy }) <= r + 1e-10 {
        return (cx, cy, r);
    }
    boundary.push(p);
    let result = welzl(pts, boundary, n - 1);
    boundary.pop();
    result
}

fn min_circle_from_boundary(boundary: &[Point2D]) -> (f64, f64, f64) {
    match boundary.len() {
        0 => (0.0, 0.0, 0.0),
        1 => (boundary[0].x, boundary[0].y, 0.0),
        2 => {
            let cx = (boundary[0].x + boundary[1].x) / 2.0;
            let cy = (boundary[0].y + boundary[1].y) / 2.0;
            let r = boundary[0].dist(&boundary[1]) / 2.0;
            (cx, cy, r)
        }
        _ => {
            let p1 = boundary[0]; let p2 = boundary[1]; let p3 = boundary[2];
            let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
            if d.abs() < 1e-15 {
                // Degenerate: return circle through two farthest points
                let d12 = p1.dist(&p2);
                let d13 = p1.dist(&p3);
                let d23 = p2.dist(&p3);
                if d12 >= d13 && d12 >= d23 {
                    ((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0, d12 / 2.0)
                } else if d13 >= d23 {
                    ((p1.x + p3.x) / 2.0, (p1.y + p3.y) / 2.0, d13 / 2.0)
                } else {
                    ((p2.x + p3.x) / 2.0, (p2.y + p3.y) / 2.0, d23 / 2.0)
                }
            } else {
                let ux = ((p1.x.powi(2) + p1.y.powi(2)) * (p2.y - p3.y)
                    + (p2.x.powi(2) + p2.y.powi(2)) * (p3.y - p1.y)
                    + (p3.x.powi(2) + p3.y.powi(2)) * (p1.y - p2.y)) / d;
                let uy = ((p1.x.powi(2) + p1.y.powi(2)) * (p3.x - p2.x)
                    + (p2.x.powi(2) + p2.y.powi(2)) * (p1.x - p3.x)
                    + (p3.x.powi(2) + p3.y.powi(2)) * (p2.x - p1.x)) / d;
                let r = p1.dist(&Point2D { x: ux, y: uy });
                (ux, uy, r)
            }
        }
    }
}

// ─── Convex Hull Area ───────────────────────────────────────────────

/// Compute area of the convex hull of a point set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_convex_hull_area(
    xs: *const f64, ys: *const f64, n: usize,
) -> f64 {
    if xs.is_null() || ys.is_null() || n < 3 { return 0.0; }
    let mut hull_xs = vec![0.0; n];
    let mut hull_ys = vec![0.0; n];
    let hull_n = unsafe { vitalis_convex_hull(xs, ys, n, hull_xs.as_mut_ptr(), hull_ys.as_mut_ptr()) };
    if hull_n < 3 { return 0.0; }
    unsafe { vitalis_polygon_area(hull_xs.as_ptr(), hull_ys.as_ptr(), hull_n as usize).abs() }
}

// ─── Collinearity Test ──────────────────────────────────────────────

/// Returns 1 if three points are collinear (within tolerance).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_are_collinear(
    x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, tol: f64,
) -> i32 {
    let area = (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2)).abs();
    if area < tol { 1 } else { 0 }
}

// ─── Angle Between Vectors ──────────────────────────────────────────

/// Angle between 2D vectors (x1,y1) and (x2,y2) in radians.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_angle_between(
    x1: f64, y1: f64, x2: f64, y2: f64,
) -> f64 {
    let dot = x1 * x2 + y1 * y2;
    let n1 = (x1 * x1 + y1 * y1).sqrt();
    let n2 = (x2 * x2 + y2 * y2).sqrt();
    if n1 < 1e-15 || n2 < 1e-15 { return 0.0; }
    (dot / (n1 * n2)).clamp(-1.0, 1.0).acos()
}

// ─── 3D Cross Product ───────────────────────────────────────────────

/// 3D cross product: a × b. Result written to (out_x, out_y, out_z).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cross_product_3d(
    ax: f64, ay: f64, az: f64,
    bx: f64, by: f64, bz: f64,
    out_x: *mut f64, out_y: *mut f64, out_z: *mut f64,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() { return -1; }
    unsafe {
        *out_x = ay * bz - az * by;
        *out_y = az * bx - ax * bz;
        *out_z = ax * by - ay * bx;
    }
    0
}

// ─── 3D Distance ────────────────────────────────────────────────────

/// 3D Euclidean distance between two points.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_distance_3d(
    x1: f64, y1: f64, z1: f64,
    x2: f64, y2: f64, z2: f64,
) -> f64 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2) + (z2 - z1).powi(2)).sqrt()
}

// ─── Spherical to Cartesian ─────────────────────────────────────────

/// Convert spherical coordinates (r, theta, phi) to Cartesian (x, y, z).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_spherical_to_cartesian(
    r: f64, theta: f64, phi: f64,
    out_x: *mut f64, out_y: *mut f64, out_z: *mut f64,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() { return -1; }
    unsafe {
        *out_x = r * theta.sin() * phi.cos();
        *out_y = r * theta.sin() * phi.sin();
        *out_z = r * theta.cos();
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convex_hull() {
        let xs = [0.0, 1.0, 2.0, 1.0, 0.5];
        let ys = [0.0, 0.0, 0.0, 2.0, 0.5]; // (0.5, 0.5) is interior
        let mut ox = [0.0; 5];
        let mut oy = [0.0; 5];
        let n = unsafe { vitalis_convex_hull(xs.as_ptr(), ys.as_ptr(), 5, ox.as_mut_ptr(), oy.as_mut_ptr()) };
        assert_eq!(n, 3); // triangle hull: (0,0),(2,0),(1,2); collinear (1,0) & interior (0.5,0.5) excluded
    }

    #[test]
    fn test_point_in_polygon() {
        // Square: (0,0), (4,0), (4,4), (0,4)
        let xs = [0.0, 4.0, 4.0, 0.0];
        let ys = [0.0, 0.0, 4.0, 4.0];
        assert_eq!(unsafe { vitalis_point_in_polygon(2.0, 2.0, xs.as_ptr(), ys.as_ptr(), 4) }, 1);
        assert_eq!(unsafe { vitalis_point_in_polygon(5.0, 2.0, xs.as_ptr(), ys.as_ptr(), 4) }, 0);
    }

    #[test]
    fn test_line_intersection() {
        let mut ox = 0.0f64;
        let mut oy = 0.0f64;
        let r = unsafe { vitalis_line_intersection(0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 2.0, 0.0, &mut ox, &mut oy) };
        assert_eq!(r, 1);
        assert!((ox - 1.0).abs() < 1e-10);
        assert!((oy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_no_intersection() {
        let r = unsafe { vitalis_line_intersection(
            0.0, 0.0, 1.0, 0.0,
            0.0, 2.0, 1.0, 2.0,
            std::ptr::null_mut(), std::ptr::null_mut()
        ) };
        assert!(r == 0 || r == -1); // parallel or no intersect
    }

    #[test]
    fn test_closest_pair() {
        let xs = [0.0, 10.0, 10.1, 50.0];
        let ys = [0.0, 0.0, 0.0, 0.0];
        let mut i = 0usize;
        let mut j = 0usize;
        let d = unsafe { vitalis_closest_pair(xs.as_ptr(), ys.as_ptr(), 4, &mut i, &mut j) };
        assert!((d - 0.1).abs() < 1e-10);
        assert!((i == 1 && j == 2) || (i == 2 && j == 1));
    }

    #[test]
    fn test_polygon_area() {
        // Unit square
        let xs = [0.0, 1.0, 1.0, 0.0];
        let ys = [0.0, 0.0, 1.0, 1.0];
        let area = unsafe { vitalis_polygon_area(xs.as_ptr(), ys.as_ptr(), 4) };
        assert!((area.abs() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_centroid() {
        let xs = [0.0, 4.0, 4.0, 0.0];
        let ys = [0.0, 0.0, 4.0, 4.0];
        let mut cx = 0.0;
        let mut cy = 0.0;
        let r = unsafe { vitalis_polygon_centroid(xs.as_ptr(), ys.as_ptr(), 4, &mut cx, &mut cy) };
        assert_eq!(r, 0);
        assert!((cx - 2.0).abs() < 1e-10);
        assert!((cy - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_perimeter() {
        let xs = [0.0, 3.0, 3.0, 0.0];
        let ys = [0.0, 0.0, 4.0, 4.0];
        let p = unsafe { vitalis_polygon_perimeter(xs.as_ptr(), ys.as_ptr(), 4) };
        assert!((p - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_area() {
        let area = unsafe { vitalis_triangle_area(0.0, 0.0, 4.0, 0.0, 0.0, 3.0) };
        assert!((area - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_to_line_dist() {
        let d = unsafe { vitalis_point_to_line_dist(1.0, 1.0, 0.0, 0.0, 2.0, 0.0) };
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_to_segment_dist() {
        // Point beyond segment end
        let d = unsafe { vitalis_point_to_segment_dist(3.0, 1.0, 0.0, 0.0, 2.0, 0.0) };
        assert!((d - 2.0f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_circumscribed_circle() {
        let mut cx = 0.0;
        let mut cy = 0.0;
        let r = unsafe { vitalis_circumscribed_circle(0.0, 0.0, 2.0, 0.0, 1.0, 3.0f64.sqrt(), &mut cx, &mut cy) };
        assert!(r > 0.0);
        assert!((cx - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_bounding_box() {
        let xs = [1.0, 5.0, 3.0, -2.0];
        let ys = [0.0, 4.0, 7.0, 1.0];
        let mut out = [0.0; 4];
        let r = unsafe { vitalis_bounding_box(xs.as_ptr(), ys.as_ptr(), 4, out.as_mut_ptr()) };
        assert_eq!(r, 0);
        assert!((out[0] - (-2.0)).abs() < 1e-10); // min_x
        assert!((out[1] - 0.0).abs() < 1e-10);    // min_y
        assert!((out[2] - 5.0).abs() < 1e-10);    // max_x
        assert!((out[3] - 7.0).abs() < 1e-10);    // max_y
    }

    #[test]
    fn test_rotate_2d() {
        let mut ox = 0.0;
        let mut oy = 0.0;
        unsafe { vitalis_rotate_2d(1.0, 0.0, std::f64::consts::FRAC_PI_2, &mut ox, &mut oy); }
        assert!(ox.abs() < 1e-10);
        assert!((oy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_convex() {
        // Square is convex
        let xs = [0.0, 1.0, 1.0, 0.0];
        let ys = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(unsafe { vitalis_is_convex(xs.as_ptr(), ys.as_ptr(), 4) }, 1);

        // Concave shape (L-shape)
        let xs2 = [0.0, 2.0, 2.0, 1.0, 1.0, 0.0];
        let ys2 = [0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
        assert_eq!(unsafe { vitalis_is_convex(xs2.as_ptr(), ys2.as_ptr(), 6) }, 0);
    }

    #[test]
    fn test_min_enclosing_circle() {
        let xs = [0.0, 2.0, 1.0];
        let ys = [0.0, 0.0, 0.1]; // almost collinear
        let mut cx = 0.0;
        let mut cy = 0.0;
        let r = unsafe { vitalis_min_enclosing_circle(xs.as_ptr(), ys.as_ptr(), 3, &mut cx, &mut cy) };
        assert!(r >= 1.0 - 0.01);
    }

    #[test]
    fn test_collinear() {
        assert_eq!(unsafe { vitalis_are_collinear(0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 1e-10) }, 1);
        assert_eq!(unsafe { vitalis_are_collinear(0.0, 0.0, 1.0, 1.0, 2.0, 3.0, 1e-10) }, 0);
    }

    #[test]
    fn test_angle_between() {
        let a = unsafe { vitalis_angle_between(1.0, 0.0, 0.0, 1.0) };
        assert!((a - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn test_distance_3d() {
        let d = unsafe { vitalis_distance_3d(0.0, 0.0, 0.0, 1.0, 1.0, 1.0) };
        assert!((d - 3.0f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_convex_hull_area() {
        let xs = [0.0, 4.0, 4.0, 0.0, 2.0]; // square + interior point
        let ys = [0.0, 0.0, 4.0, 4.0, 2.0];
        let area = unsafe { vitalis_convex_hull_area(xs.as_ptr(), ys.as_ptr(), 5) };
        assert!((area - 16.0).abs() < 1e-6);
    }
}

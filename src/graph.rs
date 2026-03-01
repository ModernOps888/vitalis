//! Graph Algorithms Module — Comprehensive graph theory implementations for Vitalis
//!
//! Pure Rust graph algorithms with zero dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - BFS (Breadth-First Search)
//! - DFS (Depth-First Search)
//! - Dijkstra's shortest path
//! - Bellman-Ford shortest path
//! - Floyd-Warshall all-pairs shortest paths
//! - Topological sort (Kahn's algorithm)
//! - Connected components (union-find)
//! - PageRank
//! - Minimum spanning tree (Kruskal's)
//! - Cycle detection
//! - Strongly connected components (Tarjan's)
//! - Bipartite checking

use std::collections::VecDeque;
use std::ffi::CString;
use std::os::raw::c_char;

// ─── Internal Graph Representation ────────────────────────────────────

/// Adjacency list graph (directed, weighted).
struct Graph {
    n: usize,
    edges: Vec<Vec<(usize, f64)>>,
}

impl Graph {
    fn new(n: usize) -> Self {
        Graph { n, edges: vec![vec![]; n] }
    }

    fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        if from < self.n && to < self.n {
            self.edges[from].push((to, weight));
        }
    }

    fn from_flat(n: usize, edge_data: &[(usize, usize, f64)]) -> Self {
        let mut g = Graph::new(n);
        for &(from, to, w) in edge_data {
            g.add_edge(from, to, w);
        }
        g
    }
}

// ─── BFS ──────────────────────────────────────────────────────────────

fn bfs(graph: &Graph, start: usize) -> Vec<i64> {
    let mut dist = vec![-1i64; graph.n];
    if start >= graph.n { return dist; }
    dist[start] = 0;
    let mut queue = VecDeque::new();
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        for &(v, _) in &graph.edges[u] {
            if dist[v] == -1 {
                dist[v] = dist[u] + 1;
                queue.push_back(v);
            }
        }
    }
    dist
}

/// BFS shortest distances from start node.
/// edges_flat: [from0,to0,from1,to1,...] (unweighted edges)
/// out_dist: pre-allocated array of n i64s, filled with distances (-1 = unreachable)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bfs(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    start: usize,
    out_dist: *mut i64,
) {
    if edges_flat.is_null() || out_dist.is_null() { return; }
    let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
    let mut g = Graph::new(n);
    for i in 0..num_edges {
        g.add_edge(ef[i*2], ef[i*2+1], 1.0);
    }
    let dist = bfs(&g, start);
    let out = unsafe { std::slice::from_raw_parts_mut(out_dist, n) };
    out.copy_from_slice(&dist);
}

// ─── DFS ──────────────────────────────────────────────────────────────

fn dfs_order(graph: &Graph, start: usize) -> Vec<usize> {
    let mut visited = vec![false; graph.n];
    let mut order = Vec::new();
    let mut stack = vec![start];
    while let Some(u) = stack.pop() {
        if u >= graph.n || visited[u] { continue; }
        visited[u] = true;
        order.push(u);
        for &(v, _) in graph.edges[u].iter().rev() {
            if !visited[v] {
                stack.push(v);
            }
        }
    }
    order
}

/// DFS traversal order from start.
/// Returns number of visited nodes, fills out_order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dfs(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    start: usize,
    out_order: *mut usize,
) -> usize {
    if edges_flat.is_null() || out_order.is_null() { return 0; }
    let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
    let mut g = Graph::new(n);
    for i in 0..num_edges {
        g.add_edge(ef[i*2], ef[i*2+1], 1.0);
    }
    let order = dfs_order(&g, start);
    let out = unsafe { std::slice::from_raw_parts_mut(out_order, n) };
    for (i, &node) in order.iter().enumerate() {
        out[i] = node;
    }
    order.len()
}

// ─── Dijkstra ─────────────────────────────────────────────────────────

fn dijkstra(graph: &Graph, start: usize) -> Vec<f64> {
    let mut dist = vec![f64::INFINITY; graph.n];
    if start >= graph.n { return dist; }
    dist[start] = 0.0;
    let mut visited = vec![false; graph.n];

    for _ in 0..graph.n {
        let mut u = graph.n;
        let mut best = f64::INFINITY;
        for i in 0..graph.n {
            if !visited[i] && dist[i] < best {
                best = dist[i];
                u = i;
            }
        }
        if u == graph.n { break; }
        visited[u] = true;
        for &(v, w) in &graph.edges[u] {
            let alt = dist[u] + w;
            if alt < dist[v] {
                dist[v] = alt;
            }
        }
    }
    dist
}

/// Dijkstra shortest paths.
/// edges_flat: [from,to,weight, from,to,weight,...] as f64 triples
/// out_dist: pre-allocated array of n f64s
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dijkstra(
    n: usize,
    edges_flat: *const f64,
    num_edges: usize,
    start: usize,
    out_dist: *mut f64,
) {
    if edges_flat.is_null() || out_dist.is_null() { return; }
    let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 3) };
    let mut g = Graph::new(n);
    for i in 0..num_edges {
        g.add_edge(ef[i*3] as usize, ef[i*3+1] as usize, ef[i*3+2]);
    }
    let d = dijkstra(&g, start);
    let out = unsafe { std::slice::from_raw_parts_mut(out_dist, n) };
    out.copy_from_slice(&d);
}

// ─── Bellman-Ford ─────────────────────────────────────────────────────

/// Bellman-Ford shortest path (handles negative weights).
/// Returns 0 on success, -1 if negative cycle detected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bellman_ford(
    n: usize,
    edges_from: *const usize,
    edges_to: *const usize,
    edges_weight: *const f64,
    num_edges: usize,
    start: usize,
    out_dist: *mut f64,
) -> i32 {
    if edges_from.is_null() || edges_to.is_null() || edges_weight.is_null() || out_dist.is_null() {
        return -1;
    }
    let from_s = unsafe { std::slice::from_raw_parts(edges_from, num_edges) };
    let to_s = unsafe { std::slice::from_raw_parts(edges_to, num_edges) };
    let w_s = unsafe { std::slice::from_raw_parts(edges_weight, num_edges) };
    let out = unsafe { std::slice::from_raw_parts_mut(out_dist, n) };

    for i in 0..n { out[i] = f64::INFINITY; }
    if start >= n { return -1; }
    out[start] = 0.0;

    for _ in 0..n-1 {
        for j in 0..num_edges {
            let u = from_s[j];
            let v = to_s[j];
            let w = w_s[j];
            if out[u] + w < out[v] {
                out[v] = out[u] + w;
            }
        }
    }
    // Check for negative cycles
    for j in 0..num_edges {
        if out[from_s[j]] + w_s[j] < out[to_s[j]] {
            return -1;
        }
    }
    0
}

// ─── Topological Sort ─────────────────────────────────────────────────

fn topological_sort(graph: &Graph) -> Option<Vec<usize>> {
    let mut in_degree = vec![0usize; graph.n];
    for u in 0..graph.n {
        for &(v, _) in &graph.edges[u] {
            in_degree[v] += 1;
        }
    }
    let mut queue: VecDeque<usize> = (0..graph.n).filter(|&i| in_degree[i] == 0).collect();
    let mut result = Vec::new();
    while let Some(u) = queue.pop_front() {
        result.push(u);
        for &(v, _) in &graph.edges[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }
    if result.len() == graph.n { Some(result) } else { None }
}

/// Topological sort via Kahn's algorithm.
/// Returns number of nodes in order (< n means cycle exists).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_toposort(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    out_order: *mut usize,
) -> usize {
    if edges_flat.is_null() || out_order.is_null() { return 0; }
    let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
    let mut g = Graph::new(n);
    for i in 0..num_edges {
        g.add_edge(ef[i*2], ef[i*2+1], 1.0);
    }
    match topological_sort(&g) {
        Some(order) => {
            let out = unsafe { std::slice::from_raw_parts_mut(out_order, n) };
            for (i, &node) in order.iter().enumerate() {
                out[i] = node;
            }
            order.len()
        }
        None => 0,
    }
}

// ─── Connected Components (Union-Find) ────────────────────────────────

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry { return false; }
        match self.rank[rx].cmp(&self.rank[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
        true
    }
}

/// Connected components via union-find.
/// Returns number of components, fills component_ids.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_connected_components(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    out_component: *mut usize,
) -> usize {
    if out_component.is_null() { return 0; }
    let mut uf = UnionFind::new(n);
    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
        for i in 0..num_edges {
            uf.union(ef[i*2], ef[i*2+1]);
        }
    }
    let out = unsafe { std::slice::from_raw_parts_mut(out_component, n) };
    let mut comp_map = std::collections::HashMap::new();
    let mut count = 0usize;
    for i in 0..n {
        let root = uf.find(i);
        let id = *comp_map.entry(root).or_insert_with(|| {
            let id = count;
            count += 1;
            id
        });
        out[i] = id;
    }
    count
}

// ─── PageRank ─────────────────────────────────────────────────────────

/// PageRank algorithm.
/// damping: typically 0.85, iterations: typically 100.
/// out_ranks: pre-allocated array of n f64s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pagerank(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    damping: f64,
    iterations: usize,
    out_ranks: *mut f64,
) {
    if out_ranks.is_null() || n == 0 { return; }
    let out = unsafe { std::slice::from_raw_parts_mut(out_ranks, n) };

    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut out_degree = vec![0usize; n];

    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
        for i in 0..num_edges {
            let from = ef[i*2];
            let to = ef[i*2+1];
            if from < n && to < n {
                adj[to].push(from); // reverse: who links TO this node
                out_degree[from] += 1;
            }
        }
    }

    let init = 1.0 / n as f64;
    let mut ranks = vec![init; n];
    let mut new_ranks = vec![0.0; n];

    for _ in 0..iterations {
        let base = (1.0 - damping) / n as f64;
        for i in 0..n {
            let mut sum = 0.0;
            for &src in &adj[i] {
                if out_degree[src] > 0 {
                    sum += ranks[src] / out_degree[src] as f64;
                }
            }
            new_ranks[i] = base + damping * sum;
        }
        std::mem::swap(&mut ranks, &mut new_ranks);
    }

    out.copy_from_slice(&ranks);
}

// ─── Kruskal's MST ───────────────────────────────────────────────────

/// Kruskal's minimum spanning tree.
/// edges: [from, to, weight, ...] triples as f64.
/// out_mst_edges: flat [from, to] pairs for MST edges.
/// Returns total MST weight.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kruskal_mst(
    n: usize,
    edges_flat: *const f64,
    num_edges: usize,
    out_mst_from: *mut usize,
    out_mst_to: *mut usize,
    out_count: *mut usize,
) -> f64 {
    if edges_flat.is_null() || n == 0 { return 0.0; }
    let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 3) };

    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..num_edges {
        edges.push((ef[i*3] as usize, ef[i*3+1] as usize, ef[i*3+2]));
    }
    edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut uf = UnionFind::new(n);
    let mut mst_count = 0usize;
    let mut total_weight = 0.0;

    for (from, to, w) in edges {
        if uf.union(from, to) {
            if !out_mst_from.is_null() && !out_mst_to.is_null() {
                unsafe {
                    *out_mst_from.add(mst_count) = from;
                    *out_mst_to.add(mst_count) = to;
                }
            }
            total_weight += w;
            mst_count += 1;
            if mst_count == n - 1 { break; }
        }
    }

    if !out_count.is_null() {
        unsafe { *out_count = mst_count; }
    }
    total_weight
}

// ─── Cycle Detection ──────────────────────────────────────────────────

/// Detect if directed graph has a cycle. Returns 1 if cycle, 0 if DAG.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_has_cycle(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
) -> i32 {
    let mut g = Graph::new(n);
    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
        for i in 0..num_edges {
            g.add_edge(ef[i*2], ef[i*2+1], 1.0);
        }
    }
    match topological_sort(&g) {
        Some(_) => 0,
        None => 1,
    }
}

// ─── Strongly Connected Components (Tarjan's) ────────────────────────

fn tarjan_scc(graph: &Graph) -> Vec<Vec<usize>> {
    let n = graph.n;
    let mut index_counter = 0usize;
    let mut stack: Vec<usize> = Vec::new();
    let mut on_stack = vec![false; n];
    let mut index = vec![usize::MAX; n];
    let mut lowlink = vec![0usize; n];
    let mut result: Vec<Vec<usize>> = Vec::new();

    fn strongconnect(
        v: usize,
        graph: &Graph,
        index_counter: &mut usize,
        stack: &mut Vec<usize>,
        on_stack: &mut Vec<bool>,
        index: &mut Vec<usize>,
        lowlink: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        index[v] = *index_counter;
        lowlink[v] = *index_counter;
        *index_counter += 1;
        stack.push(v);
        on_stack[v] = true;

        for &(w, _) in &graph.edges[v] {
            if index[w] == usize::MAX {
                strongconnect(w, graph, index_counter, stack, on_stack, index, lowlink, result);
                lowlink[v] = lowlink[v].min(lowlink[w]);
            } else if on_stack[w] {
                lowlink[v] = lowlink[v].min(index[w]);
            }
        }

        if lowlink[v] == index[v] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                component.push(w);
                if w == v { break; }
            }
            result.push(component);
        }
    }

    for v in 0..n {
        if index[v] == usize::MAX {
            strongconnect(v, graph, &mut index_counter, &mut stack, &mut on_stack,
                         &mut index, &mut lowlink, &mut result);
        }
    }
    result
}

/// Strongly connected components (Tarjan's).
/// Returns number of SCCs. out_component[i] = component ID of node i.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tarjan_scc(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
    out_component: *mut usize,
) -> usize {
    if out_component.is_null() { return 0; }
    let mut g = Graph::new(n);
    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
        for i in 0..num_edges {
            g.add_edge(ef[i*2], ef[i*2+1], 1.0);
        }
    }
    let sccs = tarjan_scc(&g);
    let out = unsafe { std::slice::from_raw_parts_mut(out_component, n) };
    for (comp_id, component) in sccs.iter().enumerate() {
        for &node in component {
            out[node] = comp_id;
        }
    }
    sccs.len()
}

// ─── Bipartite Check ──────────────────────────────────────────────────

/// Check if undirected graph is bipartite (2-colorable).
/// Returns 1 if bipartite, 0 if not.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_is_bipartite(
    n: usize,
    edges_flat: *const usize,
    num_edges: usize,
) -> i32 {
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 2) };
        for i in 0..num_edges {
            let u = ef[i*2];
            let v = ef[i*2+1];
            if u < n && v < n {
                adj[u].push(v);
                adj[v].push(u);
            }
        }
    }

    let mut color = vec![-1i32; n];
    for start in 0..n {
        if color[start] != -1 { continue; }
        color[start] = 0;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if color[v] == -1 {
                    color[v] = 1 - color[u];
                    queue.push_back(v);
                } else if color[v] == color[u] {
                    return 0;
                }
            }
        }
    }
    1
}

// ─── Floyd-Warshall ───────────────────────────────────────────────────

/// Floyd-Warshall all-pairs shortest paths.
/// dist_matrix: n*n pre-allocated f64 matrix (row-major).
/// Initialize with INFINITY for no edge, 0 on diagonal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_floyd_warshall(
    n: usize,
    edges_flat: *const f64,
    num_edges: usize,
    out_dist: *mut f64,
) {
    if out_dist.is_null() || n == 0 { return; }
    let out = unsafe { std::slice::from_raw_parts_mut(out_dist, n * n) };

    // Initialize
    for i in 0..n {
        for j in 0..n {
            out[i*n + j] = if i == j { 0.0 } else { f64::INFINITY };
        }
    }

    // Add edges
    if !edges_flat.is_null() && num_edges > 0 {
        let ef = unsafe { std::slice::from_raw_parts(edges_flat, num_edges * 3) };
        for i in 0..num_edges {
            let from = ef[i*3] as usize;
            let to = ef[i*3+1] as usize;
            let w = ef[i*3+2];
            if from < n && to < n {
                out[from*n + to] = w;
            }
        }
    }

    // Floyd-Warshall
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let through_k = out[i*n + k] + out[k*n + j];
                if through_k < out[i*n + j] {
                    out[i*n + j] = through_k;
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bfs_basic() {
        // 0→1→2→3
        let g = Graph::from_flat(4, &[(0,1,1.0),(1,2,1.0),(2,3,1.0)]);
        let dist = bfs(&g, 0);
        assert_eq!(dist, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_bfs_unreachable() {
        let g = Graph::from_flat(4, &[(0,1,1.0)]);
        let dist = bfs(&g, 0);
        assert_eq!(dist[2], -1);
        assert_eq!(dist[3], -1);
    }

    #[test]
    fn test_dfs_order() {
        // 0→1, 0→2, 1→3
        let g = Graph::from_flat(4, &[(0,1,1.0),(0,2,1.0),(1,3,1.0)]);
        let order = dfs_order(&g, 0);
        assert_eq!(order[0], 0);
        assert!(order.contains(&1));
        assert!(order.contains(&2));
        assert!(order.contains(&3));
    }

    #[test]
    fn test_dijkstra_basic() {
        // 0→1(1), 1→2(2), 0→2(5)
        let g = Graph::from_flat(3, &[(0,1,1.0),(1,2,2.0),(0,2,5.0)]);
        let dist = dijkstra(&g, 0);
        assert_eq!(dist[0], 0.0);
        assert_eq!(dist[1], 1.0);
        assert_eq!(dist[2], 3.0); // via 0→1→2
    }

    #[test]
    fn test_toposort() {
        // DAG: 0→1, 0→2, 1→3, 2→3
        let g = Graph::from_flat(4, &[(0,1,1.0),(0,2,1.0),(1,3,1.0),(2,3,1.0)]);
        let order = topological_sort(&g).unwrap();
        assert_eq!(order[0], 0);
        assert_eq!(*order.last().unwrap(), 3);
    }

    #[test]
    fn test_toposort_cycle() {
        // Has cycle: 0→1→2→0
        let g = Graph::from_flat(3, &[(0,1,1.0),(1,2,1.0),(2,0,1.0)]);
        assert!(topological_sort(&g).is_none());
    }

    #[test]
    fn test_union_find() {
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(2, 3);
        assert_eq!(uf.find(0), uf.find(1));
        assert_ne!(uf.find(0), uf.find(2));
        uf.union(1, 3);
        assert_eq!(uf.find(0), uf.find(3));
    }

    #[test]
    fn test_pagerank() {
        // Simple graph: 0→1, 1→2, 2→0
        let mut ranks = vec![0.0; 3];
        let edges: Vec<usize> = vec![0,1, 1,2, 2,0];
        unsafe {
            vitalis_pagerank(3, edges.as_ptr(), 3, 0.85, 100, ranks.as_mut_ptr());
        }
        // All nodes should have roughly equal PageRank in a cycle
        let total: f64 = ranks.iter().sum();
        assert!((total - 1.0).abs() < 0.01);
        for r in &ranks {
            assert!((*r - 1.0/3.0).abs() < 0.05);
        }
    }

    #[test]
    fn test_kruskal() {
        // Triangle: 0-1(1), 1-2(2), 0-2(3)
        let edges: Vec<f64> = vec![0.0,1.0,1.0, 1.0,2.0,2.0, 0.0,2.0,3.0];
        let mut mst_from = vec![0usize; 2];
        let mut mst_to = vec![0usize; 2];
        let mut count = 0usize;
        let w = unsafe {
            vitalis_kruskal_mst(3, edges.as_ptr(), 3, mst_from.as_mut_ptr(), mst_to.as_mut_ptr(), &mut count as *mut usize)
        };
        assert_eq!(count, 2);
        assert_eq!(w, 3.0); // 1 + 2
    }

    #[test]
    fn test_has_cycle() {
        let dag = vec![0usize,1, 1,2];
        assert_eq!(unsafe { vitalis_has_cycle(3, dag.as_ptr(), 2) }, 0);
        let cyclic = vec![0usize,1, 1,2, 2,0];
        assert_eq!(unsafe { vitalis_has_cycle(3, cyclic.as_ptr(), 3) }, 1);
    }

    #[test]
    fn test_tarjan_scc() {
        // Two SCCs: {0,1,2} cycle and {3} isolated
        let edges = vec![0usize,1, 1,2, 2,0, 2,3];
        let mut comp = vec![0usize; 4];
        let n_scc = unsafe { vitalis_tarjan_scc(4, edges.as_ptr(), 4, comp.as_mut_ptr()) };
        assert_eq!(n_scc, 2);
        // Nodes 0,1,2 should be in same component
        assert_eq!(comp[0], comp[1]);
        assert_eq!(comp[1], comp[2]);
        assert_ne!(comp[2], comp[3]);
    }

    #[test]
    fn test_bipartite() {
        // Bipartite: 0-1, 1-2 (path)
        let edges = vec![0usize,1, 1,2];
        assert_eq!(unsafe { vitalis_is_bipartite(3, edges.as_ptr(), 2) }, 1);
        // Non-bipartite: triangle 0-1, 1-2, 2-0
        let edges2 = vec![0usize,1, 1,2, 2,0];
        assert_eq!(unsafe { vitalis_is_bipartite(3, edges2.as_ptr(), 3) }, 0);
    }

    #[test]
    fn test_floyd_warshall() {
        // 0→1(1), 1→2(2), 0→2(5)
        let edges: Vec<f64> = vec![0.0,1.0,1.0, 1.0,2.0,2.0, 0.0,2.0,5.0];
        let mut dist = vec![0.0f64; 9];
        unsafe { vitalis_floyd_warshall(3, edges.as_ptr(), 3, dist.as_mut_ptr()); }
        assert_eq!(dist[0*3+1], 1.0);
        assert_eq!(dist[0*3+2], 3.0); // via 0→1→2
    }
}

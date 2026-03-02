//! Sorting & Searching Module for Vitalis v10.0
//!
//! Pure Rust implementations of classic sorting algorithms, search algorithms,
//! and selection algorithms. All functions operate on f64 arrays via FFI.

// ─── QuickSort ──────────────────────────────────────────────────────

/// In-place QuickSort (Lomuto partition). Sorts `data[0..n]` ascending.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quicksort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    quicksort_impl(arr, 0, n as isize - 1);
    0
}

fn quicksort_impl(arr: &mut [f64], lo: isize, hi: isize) {
    if lo >= hi { return; }
    let pivot = partition(arr, lo as usize, hi as usize);
    quicksort_impl(arr, lo, pivot as isize - 1);
    quicksort_impl(arr, pivot as isize + 1, hi);
}

fn partition(arr: &mut [f64], lo: usize, hi: usize) -> usize {
    // Median-of-three pivot selection
    let mid = lo + (hi - lo) / 2;
    if arr[lo] > arr[mid] { arr.swap(lo, mid); }
    if arr[lo] > arr[hi] { arr.swap(lo, hi); }
    if arr[mid] > arr[hi] { arr.swap(mid, hi); }
    arr.swap(mid, hi);
    let pivot = arr[hi];
    let mut i = lo;
    for j in lo..hi {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, hi);
    i
}

// ─── MergeSort ──────────────────────────────────────────────────────

/// Stable MergeSort. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mergesort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    let mut buf = arr.to_vec();
    mergesort_impl(arr, &mut buf, 0, n);
    0
}

fn mergesort_impl(arr: &mut [f64], buf: &mut [f64], lo: usize, hi: usize) {
    if hi - lo <= 1 { return; }
    let mid = lo + (hi - lo) / 2;
    mergesort_impl(arr, buf, lo, mid);
    mergesort_impl(arr, buf, mid, hi);
    // Merge
    buf[lo..hi].copy_from_slice(&arr[lo..hi]);
    let (mut i, mut j, mut k) = (lo, mid, lo);
    while i < mid && j < hi {
        if buf[i] <= buf[j] { arr[k] = buf[i]; i += 1; }
        else { arr[k] = buf[j]; j += 1; }
        k += 1;
    }
    while i < mid { arr[k] = buf[i]; i += 1; k += 1; }
    while j < hi { arr[k] = buf[j]; j += 1; k += 1; }
}

// ─── HeapSort ───────────────────────────────────────────────────────

/// In-place HeapSort. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_heapsort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    // Build max-heap
    for i in (0..n / 2).rev() { heapify(arr, n, i); }
    // Extract elements
    for i in (1..n).rev() {
        arr.swap(0, i);
        heapify(arr, i, 0);
    }
    0
}

fn heapify(arr: &mut [f64], n: usize, i: usize) {
    let mut largest = i;
    let left = 2 * i + 1;
    let right = 2 * i + 2;
    if left < n && arr[left] > arr[largest] { largest = left; }
    if right < n && arr[right] > arr[largest] { largest = right; }
    if largest != i {
        arr.swap(i, largest);
        heapify(arr, n, largest);
    }
}

// ─── RadixSort (for non-negative integers stored as f64) ────────────

/// Radix sort for non-negative integer values (stored as f64). Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_radixsort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };

    let mut ints: Vec<u64> = arr.iter().map(|&x| x.max(0.0) as u64).collect();
    let max_val = *ints.iter().max().unwrap_or(&0);
    if max_val == 0 { return 0; }

    let mut exp = 1u64;
    let mut output = vec![0u64; n];
    while max_val / exp > 0 {
        let mut count = [0usize; 10];
        for &val in &ints { count[((val / exp) % 10) as usize] += 1; }
        for i in 1..10 { count[i] += count[i - 1]; }
        for i in (0..n).rev() {
            let digit = ((ints[i] / exp) % 10) as usize;
            count[digit] -= 1;
            output[count[digit]] = ints[i];
        }
        ints.copy_from_slice(&output);
        exp *= 10;
    }
    for i in 0..n { arr[i] = ints[i] as f64; }
    0
}

// ─── Insertion Sort ─────────────────────────────────────────────────

/// Insertion sort — efficient for small or nearly-sorted arrays.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_insertionsort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    for i in 1..n {
        let key = arr[i];
        let mut j = i;
        while j > 0 && arr[j - 1] > key {
            arr[j] = arr[j - 1];
            j -= 1;
        }
        arr[j] = key;
    }
    0
}

// ─── Shell Sort ─────────────────────────────────────────────────────

/// Shell sort with Knuth's gap sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_shellsort(data: *mut f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    let mut gap = 1;
    while gap < n / 3 { gap = 3 * gap + 1; }
    while gap >= 1 {
        for i in gap..n {
            let key = arr[i];
            let mut j = i;
            while j >= gap && arr[j - gap] > key {
                arr[j] = arr[j - gap];
                j -= gap;
            }
            arr[j] = key;
        }
        gap /= 3;
    }
    0
}

// ─── Counting Sort ──────────────────────────────────────────────────

/// Counting sort for non-negative integers in range [0, max_val].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_countingsort(data: *mut f64, n: usize, max_val: usize) -> i32 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    let mut counts = vec![0usize; max_val + 1];
    for &val in arr.iter() {
        let v = val.max(0.0) as usize;
        if v <= max_val { counts[v] += 1; }
    }
    let mut idx = 0;
    for v in 0..=max_val {
        for _ in 0..counts[v] {
            arr[idx] = v as f64;
            idx += 1;
        }
    }
    0
}

// ─── Binary Search ──────────────────────────────────────────────────

/// Binary search in sorted array. Returns index of `target`, or -1 if not found.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_binary_search(
    data: *const f64, n: usize, target: f64,
) -> i64 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut lo = 0usize;
    let mut hi = n;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if (arr[mid] - target).abs() < 1e-12 { return mid as i64; }
        if arr[mid] < target { lo = mid + 1; } else { hi = mid; }
    }
    -1
}

/// Lower bound: first index where arr[i] >= target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lower_bound(
    data: *const f64, n: usize, target: f64,
) -> i64 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut lo = 0usize;
    let mut hi = n;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] < target { lo = mid + 1; } else { hi = mid; }
    }
    lo as i64
}

/// Upper bound: first index where arr[i] > target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_upper_bound(
    data: *const f64, n: usize, target: f64,
) -> i64 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut lo = 0usize;
    let mut hi = n;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] <= target { lo = mid + 1; } else { hi = mid; }
    }
    lo as i64
}

// ─── Interpolation Search ───────────────────────────────────────────

/// Interpolation search in uniformly distributed sorted array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_interpolation_search(
    data: *const f64, n: usize, target: f64,
) -> i64 {
    if data.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut lo = 0i64;
    let mut hi = n as i64 - 1;
    while lo <= hi && target >= arr[lo as usize] && target <= arr[hi as usize] {
        if lo == hi {
            return if (arr[lo as usize] - target).abs() < 1e-12 { lo } else { -1 };
        }
        let range = arr[hi as usize] - arr[lo as usize];
        if range.abs() < 1e-15 { return lo; }
        let pos = lo + ((target - arr[lo as usize]) / range * (hi - lo) as f64) as i64;
        let pos = pos.clamp(lo, hi);
        if (arr[pos as usize] - target).abs() < 1e-12 { return pos; }
        if arr[pos as usize] < target { lo = pos + 1; } else { hi = pos - 1; }
    }
    -1
}

// ─── QuickSelect (k-th smallest element) ────────────────────────────

/// QuickSelect: find the k-th smallest element (0-indexed).
/// Partially reorders the array. Returns the k-th value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quickselect(data: *mut f64, n: usize, k: usize) -> f64 {
    if data.is_null() || n == 0 || k >= n { return f64::NAN; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    quickselect_impl(arr, 0, n - 1, k)
}

fn quickselect_impl(arr: &mut [f64], lo: usize, hi: usize, k: usize) -> f64 {
    if lo == hi { return arr[lo]; }
    let p = partition(arr, lo, hi);
    if k == p { arr[p] }
    else if k < p { quickselect_impl(arr, lo, p.saturating_sub(1), k) }
    else { quickselect_impl(arr, p + 1, hi, k) }
}

// ─── Reservoir Sampling ─────────────────────────────────────────────

/// Reservoir sampling: select k items uniformly from stream of n.
/// Uses deterministic seeded PRNG for reproducibility.
/// Writes selected indices to `out_indices[k]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_reservoir_sample(
    n: usize, k: usize, seed: u64, out_indices: *mut usize,
) -> i32 {
    if out_indices.is_null() || k == 0 || k > n { return -1; }
    let indices = unsafe { std::slice::from_raw_parts_mut(out_indices, k) };
    for i in 0..k { indices[i] = i; }
    let mut rng = seed;
    for i in k..n {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng >> 33) as usize % (i + 1);
        if j < k { indices[j] = i; }
    }
    0
}

// ─── Is Sorted ──────────────────────────────────────────────────────

/// Check if array is sorted ascending. Returns 1 if yes, 0 if no.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_is_sorted(data: *const f64, n: usize) -> i32 {
    if data.is_null() || n <= 1 { return 1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    for i in 1..n {
        if arr[i] < arr[i - 1] { return 0; }
    }
    1
}

// ─── Inversion Count ───────────────────────────────────────────────

/// Count number of inversions in array (pairs where i < j but arr[i] > arr[j]).
/// Uses merge-sort based counting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_inversion_count(data: *const f64, n: usize) -> i64 {
    if data.is_null() || n <= 1 { return 0; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut work = arr.to_vec();
    count_inversions(&mut work, 0, n) as i64
}

fn count_inversions(arr: &mut [f64], lo: usize, hi: usize) -> usize {
    if hi - lo <= 1 { return 0; }
    let mid = lo + (hi - lo) / 2;
    let mut count = 0;
    count += count_inversions(arr, lo, mid);
    count += count_inversions(arr, mid, hi);
    // Merge and count cross-inversions
    let left = arr[lo..mid].to_vec();
    let right = arr[mid..hi].to_vec();
    let (mut i, mut j, mut k) = (0, 0, lo);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] { arr[k] = left[i]; i += 1; }
        else { arr[k] = right[j]; j += 1; count += left.len() - i; }
        k += 1;
    }
    while i < left.len() { arr[k] = left[i]; i += 1; k += 1; }
    while j < right.len() { arr[k] = right[j]; j += 1; k += 1; }
    count
}

// ─── Nth Element (partial sort) ─────────────────────────────────────

/// Partial sort: places the smallest k elements at the beginning (unsorted among themselves).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_partial_sort(data: *mut f64, n: usize, k: usize) -> i32 {
    if data.is_null() || n == 0 || k == 0 || k > n { return -1; }
    let arr = unsafe { std::slice::from_raw_parts_mut(data, n) };
    // Use a max-heap of size k
    for i in (0..k / 2).rev() { max_heapify(arr, k, i); }
    for i in k..n {
        if arr[i] < arr[0] {
            arr.swap(0, i);
            max_heapify(arr, k, 0);
        }
    }
    0
}

fn max_heapify(arr: &mut [f64], n: usize, i: usize) {
    let mut largest = i;
    let l = 2 * i + 1;
    let r = 2 * i + 2;
    if l < n && arr[l] > arr[largest] { largest = l; }
    if r < n && arr[r] > arr[largest] { largest = r; }
    if largest != i { arr.swap(i, largest); max_heapify(arr, n, largest); }
}

// ─── Rank ───────────────────────────────────────────────────────────

/// Compute rank of each element. `ranks_out[i]` = rank of data[i] (1-indexed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rank(
    data: *const f64, n: usize, ranks_out: *mut f64,
) -> i32 {
    if data.is_null() || ranks_out.is_null() || n == 0 { return -1; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let ranks = unsafe { std::slice::from_raw_parts_mut(ranks_out, n) };
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| arr[a].partial_cmp(&arr[b]).unwrap_or(std::cmp::Ordering::Equal));
    for (rank, &idx) in indices.iter().enumerate() {
        ranks[idx] = (rank + 1) as f64;
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn is_sorted_asc(arr: &[f64]) -> bool {
        arr.windows(2).all(|w| w[0] <= w[1])
    }

    #[test]
    fn test_quicksort() {
        let mut data = [5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 4.0, 6.0];
        unsafe { vitalis_quicksort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_mergesort() {
        let mut data = [5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 4.0, 6.0];
        unsafe { vitalis_mergesort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_heapsort() {
        let mut data = [5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 4.0, 6.0];
        unsafe { vitalis_heapsort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_radixsort() {
        let mut data = [170.0, 45.0, 75.0, 90.0, 802.0, 24.0, 2.0, 66.0];
        unsafe { vitalis_radixsort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_insertionsort() {
        let mut data = [5.0, 3.0, 1.0, 4.0, 2.0];
        unsafe { vitalis_insertionsort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_shellsort() {
        let mut data = [38.0, 27.0, 43.0, 3.0, 9.0, 82.0, 10.0];
        unsafe { vitalis_shellsort(data.as_mut_ptr(), data.len()); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_countingsort() {
        let mut data = [4.0, 2.0, 2.0, 8.0, 3.0, 3.0, 1.0];
        unsafe { vitalis_countingsort(data.as_mut_ptr(), data.len(), 10); }
        assert!(is_sorted_asc(&data));
    }

    #[test]
    fn test_binary_search() {
        let data = [1.0, 3.0, 5.0, 7.0, 9.0, 11.0];
        assert_eq!(unsafe { vitalis_binary_search(data.as_ptr(), 6, 7.0) }, 3);
        assert_eq!(unsafe { vitalis_binary_search(data.as_ptr(), 6, 4.0) }, -1);
    }

    #[test]
    fn test_lower_upper_bound() {
        let data = [1.0, 2.0, 2.0, 2.0, 5.0, 7.0];
        assert_eq!(unsafe { vitalis_lower_bound(data.as_ptr(), 6, 2.0) }, 1);
        assert_eq!(unsafe { vitalis_upper_bound(data.as_ptr(), 6, 2.0) }, 4);
    }

    #[test]
    fn test_interpolation_search() {
        let data = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(unsafe { vitalis_interpolation_search(data.as_ptr(), 5, 30.0) }, 2);
        assert_eq!(unsafe { vitalis_interpolation_search(data.as_ptr(), 5, 35.0) }, -1);
    }

    #[test]
    fn test_quickselect() {
        let mut data = [7.0, 3.0, 1.0, 5.0, 9.0, 2.0];
        let val = unsafe { vitalis_quickselect(data.as_mut_ptr(), 6, 2) }; // 3rd smallest
        assert!((val - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_reservoir_sample() {
        let mut indices = [0usize; 3];
        let ret = unsafe { vitalis_reservoir_sample(100, 3, 42, indices.as_mut_ptr()) };
        assert_eq!(ret, 0);
        // All indices should be valid
        for &idx in &indices { assert!(idx < 100); }
    }

    #[test]
    fn test_is_sorted() {
        let sorted = [1.0, 2.0, 3.0];
        let unsorted = [3.0, 1.0, 2.0];
        assert_eq!(unsafe { vitalis_is_sorted(sorted.as_ptr(), 3) }, 1);
        assert_eq!(unsafe { vitalis_is_sorted(unsorted.as_ptr(), 3) }, 0);
    }

    #[test]
    fn test_inversion_count() {
        let data = [2.0, 4.0, 1.0, 3.0, 5.0]; // inversions: (2,1), (4,1), (4,3) = 3
        let count = unsafe { vitalis_inversion_count(data.as_ptr(), 5) };
        assert_eq!(count, 3);
    }

    #[test]
    fn test_rank() {
        let data = [30.0, 10.0, 20.0];
        let mut ranks = [0.0; 3];
        unsafe { vitalis_rank(data.as_ptr(), 3, ranks.as_mut_ptr()); }
        assert!((ranks[0] - 3.0).abs() < 1e-10); // 30 is rank 3
        assert!((ranks[1] - 1.0).abs() < 1e-10); // 10 is rank 1
        assert!((ranks[2] - 2.0).abs() < 1e-10); // 20 is rank 2
    }
}

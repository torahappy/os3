use std::f32::consts::PI;

use num_complex::Complex;
use num_traits::ConstZero;
use rustfft::FftPlanner;

pub fn pre_emphasis_in_place(wave: &mut [f32], p: f32) {
    let mut prev = 0.0;

    for x in wave.iter_mut() {
        let current = *x; // Store original input x[n]
        *x = current - (p * prev); // Apply filter
        prev = current; // Update prev for next iteration
    }
}

pub fn check_vectors(wave: Vec<f32>, expected: Vec<f32>, epsilon: f32) {
    assert_eq!(
        wave.len(),
        expected.len(),
        "Vectors represent different lengths"
    );

    for (i, (actual, target)) in wave.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - target).abs();
        assert!(
            diff < epsilon,
            "Mismatch at index {}: actual {}, expected {} (diff: {})",
            i,
            actual,
            target,
            diff
        );
    }
}

/// Computes autocorrelation up to a specific lag order.
/// Equivalent to: r[i] = np.sum(x[0:N-i] * x[i:N])
pub fn my_autocorr(x: &[f32], order: usize) -> Vec<f32> {
    let n = x.len();
    let mut r = Vec::with_capacity(order);

    for i in 0..order {
        let mut sum = 0.0;
        // Dot product of the signal with its shifted version
        for j in 0..(n - i) {
            sum += x[j] * x[i + j];
        }
        r.push(sum);
    }
    r
}

/// The Levinson-Durbin recursion.
/// Returns (A, e) where A is the LPC coefficients and e is the prediction error.
pub fn my_levinson(signal: &[f32], order: usize) -> (Vec<f32>, f32) {
    // 1. Compute Autocorrelation (need size order + 1)
    let r = my_autocorr(signal, order + 1);

    // 2. Initialization (k = 1 case)
    // A = [1.0, -R[1] / R[0]]
    if r[0] == 0.0 {
        // Handle silence/zero-energy signal to avoid NaN
        return (vec![1.0; order + 1], 0.0);
    }

    let mut a = vec![1.0, -r[1] / r[0]];
    let mut e = r[0] + r[1] * a[1];

    // 3. Recursion
    for k in 2..=order {
        // Calculate reflection coefficient (lam)
        // Python: lam = -np.sum(A * R[k:0:-1]) / e
        // This is the dot product of current A and reversed R section
        let mut sum = 0.0;
        for j in 0..k {
            sum += a[j] * r[k - j];
        }
        let lam = -sum / e;

        // Update A
        // Python equivalent: U = [A, 0], V = lam * U[::-1], A = U + V
        // This simplifies to: new_a[i] = a[i] + lam * a[k-i]
        let mut next_a = Vec::with_capacity(k + 1);

        // Unrolled loop for clarity and performance:

        // First element is always 1.0 (a[0] + lam * 0)
        next_a.push(1.0);

        // Middle elements
        for i in 1..k {
            next_a.push(a[i] + lam * a[k - i]);
        }

        // Last element is always lam (0 + lam * a[0])
        next_a.push(lam);

        // Update state
        a = next_a;
        e = (1.0 - lam * lam) * e;
    }

    (a, e)
}

pub fn apply_hamming_in_place(buffer: &mut [f32]) {
    let m = buffer.len();
    if m <= 1 {
        return;
    }

    let denominator = (m - 1) as f32;

    for (n, sample) in buffer.iter_mut().enumerate() {
        let window_val = 0.54 - 0.46 * (2.0 * PI * n as f32 / denominator).cos();
        *sample *= window_val;
    }
}

pub fn compute_freqz(b: &f32, a: &[f32], n_fft: usize) -> Vec<Complex<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_fft);

    let mut a_padded = vec![Complex::ZERO; n_fft];
    for (i, &val) in a.iter().enumerate().take(n_fft) {
        a_padded[i] = Complex::new(val, 0.0);
    }
    fft.process(&mut a_padded);
    a_padded.iter_mut().for_each(|x| *x = *b / *x);

    return a_padded;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scipy_freqz_equivalence() {
        // --- 1. Setup Inputs ---
        let b = 100.0f32;
        let a = vec![1.0, 0.2, -0.4, 0.3, 0.6];
        let n_fft = 20;

        // --- 2. Run Rust Function ---
        let result = compute_freqz(&b, &a, n_fft);

        // --- 3. Define Expected Python Output ---
        // These values are copied directly from your scipy output
        let expected_re = [
            58.82352941,
            64.02724394,
            130.55961852,
            104.45918885,
            55.54755273,
            49.87531172,
            47.90550898,
            49.59964001,
            111.36027537,
            201.12697686,
            142.85714286,
            201.12697686,
            111.36027537,
            49.59964001,
            47.90550898,
            49.87531172,
            55.54755273,
            104.45918885,
            130.55961852,
            64.02724394,
        ];

        let expected_im = [
            0.00000000,
            33.36120750,
            106.45043900,
            -106.26855500,
            -33.11953980,
            -2.49376559,
            23.23454520,
            60.77836210,
            148.98057900,
            -12.59390060,
            -0.00000000,
            12.59390060,
            -148.98057900,
            -60.77836210,
            -23.23454520,
            2.49376559,
            33.11953980,
            106.26855500,
            -106.45043900,
            -33.36120750,
        ];

        // --- 4. Assert Equality ---
        assert_eq!(result.len(), expected_re.len());

        let epsilon = 1e-4; // Floating point tolerance

        check_vectors(
            result.iter().map(|x| x.re).collect(),
            expected_re.to_vec(),
            epsilon,
        );
        check_vectors(
            result.iter().map(|x| x.im).collect(),
            expected_im.to_vec(),
            epsilon,
        );
    }

    #[test]
    fn test_pre_emphasis() {
        // 1. Setup the input data
        let mut wave = vec![5.0, 4.0, 3.0, 9.0, 1.0, 10.0, 100.0, 1.0, 45.0, 8.0, 99.0];

        // 2. Define the expected output from Python/Scipy
        let expected = vec![
            5.0, -0.85, -0.88, 6.09, -7.73, 9.03, 90.3, -96.0, 44.03, -35.65, 91.24,
        ];

        // 3. Run the function
        pre_emphasis_in_place(&mut wave, 0.97);

        // 4. Verify results with floating point tolerance
        check_vectors(wave, expected, 1e-6);
    }

    #[test]
    fn test_hamming() {
        // 1. Input data from Python example
        let mut wave: Vec<f32> = vec![
            5.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0, 1.0, 8.0, 10.0, 2.0, 6.0, 3.0,
        ];

        // 2. Expected output from Python: numpy.hamming(13) * input
        let expected: Vec<f32> = vec![
            0.4000000000000001,
            0.0,
            0.31000000000000016,
            1.08,
            2.3100000000000005,
            2.8151150572225254,
            3.0,
            0.9383716857408418,
            6.160000000000001,
            5.4,
            0.6200000000000003,
            0.8497698855549494,
            0.24000000000000005,
        ];

        // 3. Run the function
        apply_hamming_in_place(&mut wave);

        // 4. Verify results
        check_vectors(wave, expected, 1e-6);
    }

    #[test]
    fn test_levinson() {
        // 1. Input Signal
        let signal = vec![
            2.0, 1.0, 4.0, 1.0, 3.0, 3.0, 1.0, 5.0, 1.0, 6.0, 2.0, 7.0, 3.0, 8.0, 8.0, 9.0, 1.0,
            1.0, 1.0, 2.0, 1.0, 1.0, 4.0, 1.0, 2.0, 6.0,
        ];

        // 2. Expected Output (from your Python logs)
        // Note: We supply the f64 values, Rust will cast them to f32 automatically.
        let expected_a = vec![
            1.0,
            -0.3349689538059769,
            -0.46153227671504043,
            0.11360454271664647,
            -0.13407114584170213,
            0.08143654013026622,
            0.04013509594430588,
            -0.15826580251908293,
            0.09990406933200645,
            0.010522625353963658,
            -0.14969265308352583,
        ];

        let expected_e = 174.50058965898936;

        // 3. Run Function (Order 10, because output size is 11)
        let (a, e) = my_levinson(&signal, 10);

        // 4. Verify
        check_vectors(a, expected_a, 1e-6);

        let e_diff = (e - expected_e).abs();
        assert!(
            e_diff < 1e-6,
            "Error (e) mismatch. Actual: {}, Expected: {}, Diff: {}",
            e,
            expected_e,
            e_diff
        );
    }
}

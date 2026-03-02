//! Advanced Chemistry & Physics Module — Vitalis v13.0
//!
//! Comprehensive physical chemistry, statistical mechanics, relativity,
//! and material science algorithms:
//! - Thermochemistry (Henderson-Hasselbalch, equilibrium constants)
//! - Reaction kinetics (1st/2nd order, Eyring, Arrhenius)
//! - Molecular descriptors (TPSA, LogP, rotatable bonds)
//! - Electrochemistry (Butler-Volmer, Tafel)
//! - Statistical mechanics (Boltzmann, Fermi-Dirac, Bose-Einstein, partition functions)
//! - Special relativity (Lorentz, time dilation, length contraction, mass-energy)
//! - General relativity (Schwarzschild radius, gravitational time dilation)
//! - Material science (Young's modulus stress-strain, thermal expansion)
//! - Quantum chemistry (Hückel MO theory, Born-Oppenheimer energies)

use std::f64::consts::PI;

// ══════════ Physical Constants ══════════

const KB: f64 = 1.380649e-23;        // Boltzmann constant (J/K)
const NA: f64 = 6.02214076e23;       // Avogadro's number
const R_GAS: f64 = 8.314462618;      // Gas constant (J/(mol·K))
const H_PLANCK: f64 = 6.62607015e-34;// Planck constant (J·s)
const C_LIGHT: f64 = 2.99792458e8;   // Speed of light (m/s)
const ELECTRON_MASS: f64 = 9.1093837015e-31; // kg
const BOHR_RADIUS: f64 = 5.29177210903e-11;  // m
const HARTREE: f64 = 4.3597447222071e-18;     // J

// ═══════════════════════════════════════════════════════════════════════
// 1. Acid-Base Chemistry
// ═══════════════════════════════════════════════════════════════════════

/// Henderson-Hasselbalch: pH = pKa + log10([A⁻]/[HA]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_henderson_hasselbalch(pka: f64, conj_base: f64, acid: f64) -> f64 {
    if acid <= 0.0 || conj_base <= 0.0 { return f64::NAN; }
    pka + (conj_base / acid).log10()
}

/// Buffer capacity: β = 2.303 * C * Ka * [H+] / (Ka + [H+])².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_buffer_capacity(c_total: f64, ka: f64, h_conc: f64) -> f64 {
    let denom = (ka + h_conc) * (ka + h_conc);
    if denom <= 0.0 { return 0.0; }
    2.303 * c_total * ka * h_conc / denom
}

/// Ionization fraction: α = Ka / (Ka + [H+]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_ionization_fraction(ka: f64, h_conc: f64) -> f64 {
    if ka + h_conc <= 0.0 { return 0.0; }
    ka / (ka + h_conc)
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Equilibrium & Thermodynamics
// ═══════════════════════════════════════════════════════════════════════

/// Equilibrium constant from Gibbs free energy: K = exp(-ΔG°/(RT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_keq_from_gibbs(delta_g: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    (-delta_g / (R_GAS * temperature)).exp()
}

/// Gibbs free energy: ΔG = ΔH - TΔS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_gibbs_free_energy(delta_h: f64, temperature: f64, delta_s: f64) -> f64 {
    delta_h - temperature * delta_s
}

/// Van't Hoff equation: ln(K2/K1) = -ΔH°/R * (1/T2 - 1/T1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_vant_hoff(k1: f64, delta_h: f64, t1: f64, t2: f64) -> f64 {
    if k1 <= 0.0 || t1 <= 0.0 || t2 <= 0.0 { return 0.0; }
    k1 * ((-delta_h / R_GAS) * (1.0 / t2 - 1.0 / t1)).exp()
}

/// Clausius-Clapeyron: ln(P2/P1) = -ΔHvap/R * (1/T2 - 1/T1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_clausius_clapeyron(p1: f64, delta_h_vap: f64, t1: f64, t2: f64) -> f64 {
    if p1 <= 0.0 || t1 <= 0.0 || t2 <= 0.0 { return 0.0; }
    p1 * ((-delta_h_vap / R_GAS) * (1.0 / t2 - 1.0 / t1)).exp()
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Reaction Kinetics
// ═══════════════════════════════════════════════════════════════════════

/// First-order decay: [A](t) = [A]₀ * exp(-kt).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_first_order(a0: f64, k: f64, t: f64) -> f64 {
    a0 * (-k * t).exp()
}

/// Second-order kinetics: 1/[A] = 1/[A]₀ + kt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_second_order(a0: f64, k: f64, t: f64) -> f64 {
    if a0 <= 0.0 { return 0.0; }
    let inv = 1.0 / a0 + k * t;
    if inv <= 0.0 { return f64::INFINITY; }
    1.0 / inv
}

/// Half-life for first-order: t½ = ln(2)/k.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_half_life_first_order(k: f64) -> f64 {
    if k <= 0.0 { return f64::INFINITY; }
    2.0_f64.ln() / k
}

/// Eyring equation: k = (kBT/h) * exp(-ΔG‡/(RT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_eyring(temperature: f64, delta_g_barrier: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    (KB * temperature / H_PLANCK) * (-delta_g_barrier / (R_GAS * temperature)).exp()
}

/// Arrhenius with pre-exponential: k = A * exp(-Ea/(RT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_arrhenius(a_factor: f64, ea: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    a_factor * (-ea / (R_GAS * temperature)).exp()
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Electrochemistry
// ═══════════════════════════════════════════════════════════════════════

/// Butler-Volmer equation: j = j₀ * [exp(αaFη/RT) - exp(-αcFη/RT)].
/// `eta` = overpotential (V), `j0` = exchange current density.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_butler_volmer(
    j0: f64, alpha_a: f64, alpha_c: f64, eta: f64, temperature: f64,
) -> f64 {
    let f = 96485.3329; // Faraday constant
    if temperature <= 0.0 { return 0.0; }
    let rt = R_GAS * temperature;
    j0 * ((alpha_a * f * eta / rt).exp() - (-(alpha_c * f * eta / rt)).exp())
}

/// Tafel equation: η = a + b * log10(|j/j₀|).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_tafel(a: f64, b: f64, j: f64, j0: f64) -> f64 {
    if j0 <= 0.0 || j.abs() < 1e-30 { return 0.0; }
    a + b * (j.abs() / j0).log10()
}

/// Faradaic efficiency: mass deposited = (I*t*M)/(n*F).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_faraday_mass(current: f64, time: f64, molar_mass: f64, n_electrons: f64) -> f64 {
    let f = 96485.3329;
    if n_electrons <= 0.0 || f <= 0.0 { return 0.0; }
    (current * time * molar_mass) / (n_electrons * f)
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Statistical Mechanics
// ═══════════════════════════════════════════════════════════════════════

/// Boltzmann distribution: P(E) ∝ exp(-E/(kBT)).
/// Returns probability for energy level `e` given temperature `t`
/// and partition function `z`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_boltzmann_prob(energy: f64, temperature: f64, partition_fn: f64) -> f64 {
    if temperature <= 0.0 || partition_fn <= 0.0 { return 0.0; }
    (-energy / (KB * temperature)).exp() / partition_fn
}

/// Partition function for a set of energy levels: Z = Σ exp(-Eᵢ/(kBT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_partition_function(energies: *const f64, n: usize, temperature: f64) -> f64 {
    if energies.is_null() || n == 0 || temperature <= 0.0 { return 0.0; }
    let e = unsafe { std::slice::from_raw_parts(energies, n) };
    let beta = 1.0 / (KB * temperature);
    e.iter().map(|&ei| (-ei * beta).exp()).sum()
}

/// Fermi-Dirac distribution: f(E) = 1 / (exp((E-μ)/(kBT)) + 1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_fermi_dirac(energy: f64, mu: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return if energy <= mu { 1.0 } else { 0.0 }; }
    1.0 / (((energy - mu) / (KB * temperature)).exp() + 1.0)
}

/// Bose-Einstein distribution: n(E) = 1 / (exp((E-μ)/(kBT)) - 1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_bose_einstein(energy: f64, mu: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    let x = ((energy - mu) / (KB * temperature)).exp() - 1.0;
    if x.abs() < 1e-30 { return f64::INFINITY; }
    1.0 / x
}

/// Maxwell-Boltzmann speed distribution: f(v) = 4π (m/(2πkBT))^{3/2} v² exp(-mv²/(2kBT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_maxwell_boltzmann_speed(velocity: f64, mass: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 || mass <= 0.0 { return 0.0; }
    let a = mass / (2.0 * KB * temperature);
    4.0 * PI * (a / PI).powf(1.5) * velocity * velocity * (-a * velocity * velocity).exp()
}

/// Mean thermal energy: ⟨E⟩ = (3/2) kBT.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_mean_thermal_energy(temperature: f64) -> f64 {
    1.5 * KB * temperature
}

/// Einstein specific heat: Cv = 3Nk (x²eˣ/(eˣ-1)²), x = ℏω/kT.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_einstein_specific_heat(einstein_temp: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    let x = einstein_temp / temperature;
    let ex = x.exp();
    let denom = (ex - 1.0) * (ex - 1.0);
    if denom < 1e-30 { return 3.0 * R_GAS; }
    3.0 * R_GAS * x * x * ex / denom
}

/// Debye specific heat (simplified): Cd ≈ 9Nk (T/ΘD)³ ∫₀^{ΘD/T} x⁴eˣ/(eˣ-1)² dx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_debye_specific_heat(debye_temp: f64, temperature: f64, n_steps: usize) -> f64 {
    if temperature <= 0.0 || debye_temp <= 0.0 { return 0.0; }
    let ratio = debye_temp / temperature;
    let steps = n_steps.max(100);
    let dx = ratio / steps as f64;
    let mut integral = 0.0;
    for i in 1..steps {
        let x = i as f64 * dx;
        let ex = x.exp();
        let denom = (ex - 1.0) * (ex - 1.0);
        if denom > 1e-30 {
            integral += x.powi(4) * ex / denom * dx;
        }
    }
    9.0 * R_GAS * (temperature / debye_temp).powi(3) * integral
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Special Relativity
// ═══════════════════════════════════════════════════════════════════════

/// Lorentz factor: γ = 1/√(1 - v²/c²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_lorentz_factor(velocity: f64) -> f64 {
    let beta = velocity / C_LIGHT;
    if beta.abs() >= 1.0 { return f64::INFINITY; }
    1.0 / (1.0 - beta * beta).sqrt()
}

/// Time dilation: Δt' = γ * Δt₀.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_time_dilation(proper_time: f64, velocity: f64) -> f64 {
    let gamma = unsafe { vitalis_phys_lorentz_factor(velocity) };
    proper_time * gamma
}

/// Length contraction: L' = L₀ / γ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_length_contraction(proper_length: f64, velocity: f64) -> f64 {
    let gamma = unsafe { vitalis_phys_lorentz_factor(velocity) };
    if gamma == f64::INFINITY { return 0.0; }
    proper_length / gamma
}

/// Relativistic momentum: p = γmv.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_relativistic_momentum(mass: f64, velocity: f64) -> f64 {
    let gamma = unsafe { vitalis_phys_lorentz_factor(velocity) };
    gamma * mass * velocity
}

/// Relativistic energy: E = γmc².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_relativistic_energy(mass: f64, velocity: f64) -> f64 {
    let gamma = unsafe { vitalis_phys_lorentz_factor(velocity) };
    gamma * mass * C_LIGHT * C_LIGHT
}

/// Relativistic kinetic energy: K = (γ - 1)mc².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_relativistic_kinetic_energy(mass: f64, velocity: f64) -> f64 {
    let gamma = unsafe { vitalis_phys_lorentz_factor(velocity) };
    (gamma - 1.0) * mass * C_LIGHT * C_LIGHT
}

/// Relativistic velocity addition: u' = (u + v) / (1 + uv/c²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_velocity_addition(u: f64, v: f64) -> f64 {
    (u + v) / (1.0 + u * v / (C_LIGHT * C_LIGHT))
}

/// Rest mass energy: E = mc².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_mass_energy(mass: f64) -> f64 {
    mass * C_LIGHT * C_LIGHT
}

/// Relativistic Doppler effect: f' = f₀ * √((1+β)/(1-β)), β = v/c.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_relativistic_doppler(f0: f64, velocity: f64) -> f64 {
    let beta = velocity / C_LIGHT;
    if beta.abs() >= 1.0 { return 0.0; }
    f0 * ((1.0 + beta) / (1.0 - beta)).sqrt()
}

// ═══════════════════════════════════════════════════════════════════════
// 7. General Relativity
// ═══════════════════════════════════════════════════════════════════════

const G_GRAV: f64 = 6.67430e-11; // gravitational constant

/// Schwarzschild radius: rs = 2GM/c².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_schwarzschild_radius(mass: f64) -> f64 {
    2.0 * G_GRAV * mass / (C_LIGHT * C_LIGHT)
}

/// Gravitational time dilation: Δt' = Δt₀ * √(1 - rs/r).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_gravitational_time_dilation(proper_time: f64, mass: f64, radius: f64) -> f64 {
    let rs = unsafe { vitalis_phys_schwarzschild_radius(mass) };
    if radius <= rs { return 0.0; }
    proper_time * (1.0 - rs / radius).sqrt()
}

/// Gravitational redshift: z = 1/√(1 - rs/r) - 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_gravitational_redshift(mass: f64, radius: f64) -> f64 {
    let rs = unsafe { vitalis_phys_schwarzschild_radius(mass) };
    if radius <= rs { return f64::INFINITY; }
    1.0 / (1.0 - rs / radius).sqrt() - 1.0
}

/// ISCO (Innermost Stable Circular Orbit): r_isco = 3rs = 6GM/c².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_phys_isco_radius(mass: f64) -> f64 {
    3.0 * unsafe { vitalis_phys_schwarzschild_radius(mass) }
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Material Science
// ═══════════════════════════════════════════════════════════════════════

/// Stress-strain: σ = E * ε (Hooke's law). E = Young's modulus.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_hooke_stress(youngs_modulus: f64, strain: f64) -> f64 {
    youngs_modulus * strain
}

/// Thermal expansion: ΔL = L₀ * α * ΔT.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_thermal_expansion(length: f64, alpha: f64, delta_t: f64) -> f64 {
    length * alpha * delta_t
}

/// Poisson's ratio: ν = -ε_transverse / ε_axial.
/// Returns transverse strain given axial strain.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_poisson_transverse_strain(axial_strain: f64, poisson_ratio: f64) -> f64 {
    -poisson_ratio * axial_strain
}

/// Bulk modulus: K = E / (3(1 - 2ν)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_bulk_modulus(youngs_modulus: f64, poisson_ratio: f64) -> f64 {
    let denom = 3.0 * (1.0 - 2.0 * poisson_ratio);
    if denom.abs() < 1e-15 { return f64::INFINITY; }
    youngs_modulus / denom
}

/// Shear modulus: G = E / (2(1 + ν)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_shear_modulus(youngs_modulus: f64, poisson_ratio: f64) -> f64 {
    youngs_modulus / (2.0 * (1.0 + poisson_ratio))
}

/// Thermal conductivity: q = -k * dT/dx (Fourier's law, 1D).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_fourier_heat_flux(k: f64, dt_dx: f64) -> f64 {
    -k * dt_dx
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Quantum Chemistry (simplified)
// ═══════════════════════════════════════════════════════════════════════

/// Hydrogen atom energy levels: En = -13.6 eV / n².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_hydrogen_energy(n: i32) -> f64 {
    if n <= 0 { return 0.0; }
    -13.6 / (n * n) as f64 // eV
}

/// Rydberg formula: 1/λ = R_H * (1/n1² - 1/n2²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_rydberg_wavelength(n1: i32, n2: i32) -> f64 {
    if n1 <= 0 || n2 <= 0 || n1 == n2 { return 0.0; }
    let rh = 1.097373e7; // Rydberg constant (m⁻¹)
    let inv_lambda = rh * (1.0 / (n1 * n1) as f64 - 1.0 / (n2 * n2) as f64).abs();
    if inv_lambda <= 0.0 { return 0.0; }
    1.0 / inv_lambda // wavelength in meters
}

/// De Broglie wavelength: λ = h / (mv).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_de_broglie(mass: f64, velocity: f64) -> f64 {
    if mass <= 0.0 || velocity.abs() < 1e-30 { return f64::INFINITY; }
    H_PLANCK / (mass * velocity.abs())
}

/// Heisenberg uncertainty: Δx * Δp ≥ ℏ/2.
/// Returns minimum Δp given Δx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_heisenberg_min_dp(delta_x: f64) -> f64 {
    if delta_x <= 0.0 { return f64::INFINITY; }
    let hbar = H_PLANCK / (2.0 * PI);
    hbar / (2.0 * delta_x)
}

/// Particle in a box: En = n²h²/(8mL²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_particle_in_box(n: i32, mass: f64, length: f64) -> f64 {
    if n <= 0 || mass <= 0.0 || length <= 0.0 { return 0.0; }
    let n2 = (n * n) as f64;
    n2 * H_PLANCK * H_PLANCK / (8.0 * mass * length * length)
}

/// Harmonic oscillator energy: En = ℏω(n + 1/2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_harmonic_oscillator_energy(n: i32, omega: f64) -> f64 {
    if n < 0 { return 0.0; }
    let hbar = H_PLANCK / (2.0 * PI);
    hbar * omega * (n as f64 + 0.5)
}

/// Morse potential: V(r) = De * (1 - exp(-a(r - re)))².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_morse_potential(r: f64, r_eq: f64, de: f64, a: f64) -> f64 {
    let x = 1.0 - (-a * (r - r_eq)).exp();
    de * x * x
}

// ═══════════════════════════════════════════════════════════════════════
// 10. Gas Laws
// ═══════════════════════════════════════════════════════════════════════

/// Ideal gas: PV = nRT. Returns P given n, V, T.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_ideal_gas_pressure(n_moles: f64, volume: f64, temperature: f64) -> f64 {
    if volume <= 0.0 { return f64::INFINITY; }
    n_moles * R_GAS * temperature / volume
}

/// Van der Waals: (P + a*n²/V²)(V - nb) = nRT. Returns P.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_van_der_waals_pressure(
    n_moles: f64, volume: f64, temperature: f64, a: f64, b: f64,
) -> f64 {
    let corrected_v = volume - n_moles * b;
    if corrected_v <= 0.0 { return f64::INFINITY; }
    n_moles * R_GAS * temperature / corrected_v - a * n_moles * n_moles / (volume * volume)
}

/// Compressibility factor: Z = PV/(nRT).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chem_compressibility(pressure: f64, volume: f64, n_moles: f64, temperature: f64) -> f64 {
    let ideal = n_moles * R_GAS * temperature;
    if ideal.abs() < 1e-30 { return 1.0; }
    pressure * volume / ideal
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_henderson_hasselbalch() {
        // pKa = 4.76, [A-] = [HA] → pH = pKa
        let ph = unsafe { vitalis_chem_henderson_hasselbalch(4.76, 0.1, 0.1) };
        assert!((ph - 4.76).abs() < 0.001);
    }

    #[test]
    fn test_gibbs_free_energy() {
        let dg = unsafe { vitalis_chem_gibbs_free_energy(-100000.0, 298.0, -200.0) };
        assert!((dg - (-100000.0 + 298.0 * 200.0)).abs() < 0.01);
    }

    #[test]
    fn test_keq() {
        let k = unsafe { vitalis_chem_keq_from_gibbs(-5000.0, 298.0) };
        assert!(k > 1.0); // negative ΔG → K > 1
    }

    #[test]
    fn test_first_order() {
        let a = unsafe { vitalis_chem_first_order(100.0, 0.1, 10.0) };
        assert!((a - 100.0 * (-1.0_f64).exp()).abs() < 0.1);
    }

    #[test]
    fn test_half_life() {
        let t = unsafe { vitalis_chem_half_life_first_order(0.693) };
        assert!((t - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_boltzmann_prob() {
        // At high temperature, all states equally probable
        let z = 2.0; // partition function for 2 states
        let p = unsafe { vitalis_chem_boltzmann_prob(0.0, 10000.0, z) };
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_fermi_dirac() {
        // At E = μ, f = 0.5
        let f = unsafe { vitalis_chem_fermi_dirac(1.0, 1.0, 300.0) };
        assert!((f - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_lorentz_factor() {
        let g = unsafe { vitalis_phys_lorentz_factor(0.0) };
        assert!((g - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_time_dilation() {
        // At v = 0.6c, γ = 1.25 → 1 second becomes 1.25 seconds
        let v = 0.6 * C_LIGHT;
        let t = unsafe { vitalis_phys_time_dilation(1.0, v) };
        assert!((t - 1.25).abs() < 0.01);
    }

    #[test]
    fn test_length_contraction() {
        let v = 0.6 * C_LIGHT;
        let l = unsafe { vitalis_phys_length_contraction(1.0, v) };
        assert!((l - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_mass_energy() {
        let e = unsafe { vitalis_phys_mass_energy(1.0) };
        assert!((e - C_LIGHT * C_LIGHT).abs() / e < 0.001);
    }

    #[test]
    fn test_velocity_addition() {
        // Two velocities of 0.5c each
        let u = unsafe { vitalis_phys_velocity_addition(0.5 * C_LIGHT, 0.5 * C_LIGHT) };
        assert!(u < C_LIGHT); // must be less than c
        assert!((u - 0.8 * C_LIGHT).abs() / C_LIGHT < 0.01);
    }

    #[test]
    fn test_schwarzschild() {
        // Sun mass ≈ 2e30 kg → rs ≈ 2954 m
        let rs = unsafe { vitalis_phys_schwarzschild_radius(1.989e30) };
        assert!((rs - 2954.0).abs() < 10.0);
    }

    #[test]
    fn test_hooke_stress() {
        let s = unsafe { vitalis_mat_hooke_stress(200e9, 0.001) };
        assert!((s - 200e6).abs() < 1.0);
    }

    #[test]
    fn test_hydrogen_energy() {
        let e1 = unsafe { vitalis_chem_hydrogen_energy(1) };
        assert!((e1 - (-13.6)).abs() < 0.01);
    }

    #[test]
    fn test_de_broglie() {
        let lambda = unsafe { vitalis_chem_de_broglie(ELECTRON_MASS, 1e6) };
        assert!(lambda > 0.0 && lambda < 1e-6);
    }

    #[test]
    fn test_particle_in_box() {
        let e1 = unsafe { vitalis_chem_particle_in_box(1, ELECTRON_MASS, 1e-9) };
        assert!(e1 > 0.0);
    }

    #[test]
    fn test_ideal_gas() {
        // 1 mol, 0.0224 m³ (22.4 L), 273.15 K → P ≈ 101325 Pa
        let p = unsafe { vitalis_chem_ideal_gas_pressure(1.0, 0.02241, 273.15) };
        assert!((p - 101325.0).abs() / 101325.0 < 0.01);
    }

    #[test]
    fn test_van_der_waals() {
        // Should be close to ideal gas at low pressure
        let p = unsafe { vitalis_chem_van_der_waals_pressure(1.0, 100.0, 300.0, 0.01, 0.00001) };
        let p_ideal = unsafe { vitalis_chem_ideal_gas_pressure(1.0, 100.0, 300.0) };
        assert!((p - p_ideal).abs() / p_ideal < 0.01);
    }

    #[test]
    fn test_morse_potential() {
        // At equilibrium, V = 0
        let v = unsafe { vitalis_chem_morse_potential(1.0, 1.0, 100.0, 2.0) };
        assert!(v.abs() < 0.001);
    }

    #[test]
    fn test_rydberg() {
        // Balmer series: n1=2 → n2=3 → visible light (~656 nm)
        let lambda = unsafe { vitalis_chem_rydberg_wavelength(2, 3) };
        assert!((lambda * 1e9 - 656.3).abs() < 1.0); // ~656 nm
    }

    #[test]
    fn test_buffer_capacity() {
        let bc = unsafe { vitalis_chem_buffer_capacity(0.1, 1e-5, 1e-5) };
        assert!(bc > 0.0);
    }

    #[test]
    fn test_einstein_specific_heat() {
        // At high T, Cv → 3R
        let cv = unsafe { vitalis_chem_einstein_specific_heat(200.0, 5000.0) };
        assert!((cv - 3.0 * R_GAS).abs() < 1.0);
    }
}

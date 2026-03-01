//! Science & Physics Module for Vitalis v9.0
//!
//! Pure Rust implementations of physics, chemistry, and scientific constants.
//! Zero external dependencies.
//!
//! # Domains:
//! - **Constants**: Speed of light, Planck's constant, Boltzmann, Avogadro, etc.
//! - **Mechanics**: Kinematic equations, projectile motion, pendulum, orbital velocity
//! - **Thermodynamics**: Ideal gas, Carnot efficiency, Stefan-Boltzmann, heat transfer
//! - **Electromagnetism**: Coulomb's law, capacitance, inductance, Ohm's law
//! - **Waves & Optics**: Wavelength-frequency, Doppler shift, Snell's law, diffraction
//! - **Nuclear / Radioactive decay**: Half-life, decay constant, activity
//! - **Chemistry**: pH, molar mass helpers, Arrhenius equation, Nernst equation
//! - **Astrophysics**: Schwarzschild radius, luminosity, Hubble's law, escape velocity
//! - **Fluid dynamics**: Reynolds number, Bernoulli, drag force
//! - **Dimensional analysis**: Unit conversion helpers

use std::f64::consts::PI;

// ─── Physical Constants ───────────────────────────────────────────────

pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;       // m/s
pub const PLANCK: f64 = 6.626_070_15e-34;             // J·s
pub const HBAR: f64 = 1.054_571_817e-34;              // J·s (ℏ = h/2π)
pub const BOLTZMANN: f64 = 1.380_649e-23;             // J/K
pub const AVOGADRO: f64 = 6.022_140_76e23;            // 1/mol
pub const GAS_CONSTANT: f64 = 8.314_462_618;          // J/(mol·K)
pub const GRAVITATIONAL: f64 = 6.674_30e-11;          // m³/(kg·s²)
pub const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19; // C
pub const ELECTRON_MASS: f64 = 9.109_383_7015e-31;    // kg
pub const PROTON_MASS: f64 = 1.672_621_923_69e-27;    // kg
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;   // W/(m²·K⁴)
pub const COULOMB_CONST: f64 = 8.987_551_7923e9;      // N·m²/C²
pub const VACUUM_PERMITTIVITY: f64 = 8.854_187_8128e-12; // F/m
pub const VACUUM_PERMEABILITY: f64 = 1.256_637_062_12e-6; // H/m

/// Get a physical constant by name (returns 0.0 if unknown).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_constant(name: *const u8, len: usize) -> f64 {
    if name.is_null() || len == 0 { return 0.0; }
    let bytes = unsafe { std::slice::from_raw_parts(name, len) };
    let s = std::str::from_utf8(bytes).unwrap_or("");
    match s {
        "c" | "speed_of_light" => SPEED_OF_LIGHT,
        "h" | "planck" => PLANCK,
        "hbar" => HBAR,
        "k" | "boltzmann" => BOLTZMANN,
        "Na" | "avogadro" => AVOGADRO,
        "R" | "gas_constant" => GAS_CONSTANT,
        "G" | "gravitational" => GRAVITATIONAL,
        "e" | "elementary_charge" => ELEMENTARY_CHARGE,
        "me" | "electron_mass" => ELECTRON_MASS,
        "mp" | "proton_mass" => PROTON_MASS,
        "sigma" | "stefan_boltzmann" => STEFAN_BOLTZMANN,
        "ke" | "coulomb_constant" => COULOMB_CONST,
        "epsilon0" | "vacuum_permittivity" => VACUUM_PERMITTIVITY,
        "mu0" | "vacuum_permeability" => VACUUM_PERMEABILITY,
        _ => 0.0,
    }
}

// ─── Mechanics ────────────────────────────────────────────────────────

/// Kinematic: final velocity v = v0 + a*t.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kinematic_v(v0: f64, a: f64, t: f64) -> f64 {
    v0 + a * t
}

/// Kinematic: displacement s = v0*t + 0.5*a*t².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kinematic_s(v0: f64, a: f64, t: f64) -> f64 {
    v0 * t + 0.5 * a * t * t
}

/// Kinematic: v² = v0² + 2*a*s → solve for v.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kinematic_v_from_s(v0: f64, a: f64, s: f64) -> f64 {
    (v0 * v0 + 2.0 * a * s).abs().sqrt()
}

/// Kinetic energy KE = 0.5 * m * v².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kinetic_energy(mass: f64, velocity: f64) -> f64 {
    0.5 * mass * velocity * velocity
}

/// Gravitational potential energy PE = m * g * h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_potential_energy(mass: f64, g: f64, height: f64) -> f64 {
    mass * g * height
}

/// Simple pendulum period T = 2π√(L/g).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pendulum_period(length: f64, g: f64) -> f64 {
    if g <= 0.0 || length <= 0.0 { return 0.0; }
    2.0 * PI * (length / g).sqrt()
}

/// Orbital velocity v = √(GM/r).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_orbital_velocity(mass: f64, radius: f64) -> f64 {
    if radius <= 0.0 { return 0.0; }
    (GRAVITATIONAL * mass / radius).sqrt()
}

/// Escape velocity v = √(2GM/r).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_escape_velocity(mass: f64, radius: f64) -> f64 {
    if radius <= 0.0 { return 0.0; }
    (2.0 * GRAVITATIONAL * mass / radius).sqrt()
}

/// Projectile range R = v²sin(2θ)/g.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_projectile_range(v: f64, theta: f64, g: f64) -> f64 {
    if g <= 0.0 { return 0.0; }
    v * v * (2.0 * theta).sin() / g
}

/// Projectile max height H = v²sin²(θ)/(2g).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_projectile_max_height(v: f64, theta: f64, g: f64) -> f64 {
    if g <= 0.0 { return 0.0; }
    let s = theta.sin();
    v * v * s * s / (2.0 * g)
}

// ─── Thermodynamics ──────────────────────────────────────────────────

/// Ideal gas law: PV = nRT → solve for P.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ideal_gas_pressure(n: f64, t: f64, v: f64) -> f64 {
    if v <= 0.0 { return 0.0; }
    n * GAS_CONSTANT * t / v
}

/// Ideal gas law: solve for T = PV/(nR).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ideal_gas_temperature(p: f64, v: f64, n: f64) -> f64 {
    if n <= 0.0 { return 0.0; }
    p * v / (n * GAS_CONSTANT)
}

/// Carnot efficiency η = 1 - T_cold/T_hot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_carnot_efficiency(t_cold: f64, t_hot: f64) -> f64 {
    if t_hot <= 0.0 || t_cold < 0.0 { return 0.0; }
    1.0 - t_cold / t_hot
}

/// Stefan-Boltzmann radiation power P = σ * A * T⁴.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_radiation_power(area: f64, temp: f64) -> f64 {
    STEFAN_BOLTZMANN * area * temp.powi(4)
}

/// Heat transfer Q = m * c * ΔT.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_heat_transfer(mass: f64, specific_heat: f64, delta_t: f64) -> f64 {
    mass * specific_heat * delta_t
}

/// Entropy change ΔS = Q / T.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_entropy_change(heat: f64, temperature: f64) -> f64 {
    if temperature <= 0.0 { return 0.0; }
    heat / temperature
}

// ─── Electromagnetism ────────────────────────────────────────────────

/// Coulomb's law: F = ke * q1 * q2 / r².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_coulomb_force(q1: f64, q2: f64, r: f64) -> f64 {
    if r.abs() < 1e-30 { return 0.0; }
    COULOMB_CONST * q1 * q2 / (r * r)
}

/// Electric field E = ke * q / r².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_electric_field(q: f64, r: f64) -> f64 {
    if r.abs() < 1e-30 { return 0.0; }
    COULOMB_CONST * q / (r * r)
}

/// Ohm's law: V = I * R.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ohms_law_v(current: f64, resistance: f64) -> f64 {
    current * resistance
}

/// Electrical power P = V * I.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_electrical_power(voltage: f64, current: f64) -> f64 {
    voltage * current
}

/// Capacitor energy E = 0.5 * C * V².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_capacitor_energy(capacitance: f64, voltage: f64) -> f64 {
    0.5 * capacitance * voltage * voltage
}

/// Magnetic force F = qvB sin(θ).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_magnetic_force(q: f64, v: f64, b: f64, theta: f64) -> f64 {
    q * v * b * theta.sin()
}

// ─── Waves & Optics ──────────────────────────────────────────────────

/// Wavelength-frequency relation: c = λν → λ = c/ν.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_wavelength(frequency: f64) -> f64 {
    if frequency.abs() < 1e-30 { return 0.0; }
    SPEED_OF_LIGHT / frequency
}

/// Photon energy E = hν.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_photon_energy(frequency: f64) -> f64 {
    PLANCK * frequency
}

/// Doppler shift: f_observed = f_source * (v_sound + v_observer) / (v_sound + v_source).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_doppler(f_source: f64, v_sound: f64, v_observer: f64, v_source: f64) -> f64 {
    let denom = v_sound + v_source;
    if denom.abs() < 1e-30 { return 0.0; }
    f_source * (v_sound + v_observer) / denom
}

/// Snell's law: n1*sin(θ1) = n2*sin(θ2) → θ2.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_snell(n1: f64, theta1: f64, n2: f64) -> f64 {
    if n2.abs() < 1e-30 { return 0.0; }
    let sin_theta2 = n1 * theta1.sin() / n2;
    if sin_theta2.abs() > 1.0 { return f64::NAN; } // Total internal reflection
    sin_theta2.asin()
}

/// De Broglie wavelength: λ = h / (m*v).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_de_broglie(mass: f64, velocity: f64) -> f64 {
    let p = mass * velocity;
    if p.abs() < 1e-50 { return 0.0; }
    PLANCK / p
}

// ─── Nuclear / Radioactive Decay ─────────────────────────────────────

/// Decay constant λ = ln(2) / half_life.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_decay_constant(half_life: f64) -> f64 {
    if half_life <= 0.0 { return 0.0; }
    2.0f64.ln() / half_life
}

/// Radioactive decay: N(t) = N0 * e^(-λt).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_radioactive_decay(n0: f64, decay_const: f64, t: f64) -> f64 {
    n0 * (-decay_const * t).exp()
}

/// Activity A = λ * N.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_activity(decay_const: f64, n: f64) -> f64 {
    decay_const * n
}

/// Mass-energy equivalence E = mc².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mass_energy(mass: f64) -> f64 {
    mass * SPEED_OF_LIGHT * SPEED_OF_LIGHT
}

// ─── Chemistry ───────────────────────────────────────────────────────

/// pH = -log₁₀([H⁺]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ph(h_concentration: f64) -> f64 {
    if h_concentration <= 0.0 { return 14.0; }
    -h_concentration.log10()
}

/// pOH = 14 - pH.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_poh(ph: f64) -> f64 {
    14.0 - ph
}

/// Arrhenius equation: k = A * e^(-Ea/(RT)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_arrhenius(a_factor: f64, ea: f64, t: f64) -> f64 {
    if t <= 0.0 { return 0.0; }
    a_factor * (-ea / (GAS_CONSTANT * t)).exp()
}

/// Nernst equation: E = E0 - (RT/(nF)) * ln(Q).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_nernst(e0: f64, n_electrons: f64, temperature: f64, q: f64) -> f64 {
    if n_electrons.abs() < 1e-30 || q <= 0.0 { return e0; }
    let f = 96_485.332_12; // Faraday constant C/mol
    e0 - (GAS_CONSTANT * temperature / (n_electrons * f)) * q.ln()
}

/// Dilution: M1*V1 = M2*V2 → solve for V2.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dilution(m1: f64, v1: f64, m2: f64) -> f64 {
    if m2.abs() < 1e-30 { return 0.0; }
    m1 * v1 / m2
}

// ─── Astrophysics ────────────────────────────────────────────────────

/// Schwarzschild radius r_s = 2GM/c².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_schwarzschild_radius(mass: f64) -> f64 {
    2.0 * GRAVITATIONAL * mass / (SPEED_OF_LIGHT * SPEED_OF_LIGHT)
}

/// Luminosity L = 4πR²σT⁴.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_luminosity(radius: f64, temperature: f64) -> f64 {
    4.0 * PI * radius * radius * STEFAN_BOLTZMANN * temperature.powi(4)
}

/// Hubble's law: v = H0 * d (velocity from distance).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hubble_velocity(h0: f64, distance: f64) -> f64 {
    h0 * distance
}

/// Redshift: z = (λ_obs - λ_emit) / λ_emit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_redshift(lambda_obs: f64, lambda_emit: f64) -> f64 {
    if lambda_emit.abs() < 1e-30 { return 0.0; }
    (lambda_obs - lambda_emit) / lambda_emit
}

// ─── Fluid Dynamics ──────────────────────────────────────────────────

/// Reynolds number Re = ρvL/μ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_reynolds_number(density: f64, velocity: f64, length: f64, viscosity: f64) -> f64 {
    if viscosity.abs() < 1e-30 { return 0.0; }
    density * velocity * length / viscosity
}

/// Drag force F = 0.5 * Cd * ρ * A * v².
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_drag_force(cd: f64, density: f64, area: f64, velocity: f64) -> f64 {
    0.5 * cd * density * area * velocity * velocity
}

/// Bernoulli: P + 0.5*ρv² + ρgh = const → solve for P.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bernoulli_pressure(density: f64, velocity: f64, g: f64, height: f64, p_total: f64) -> f64 {
    p_total - 0.5 * density * velocity * velocity - density * g * height
}

// ─── Unit Conversion ─────────────────────────────────────────────────

/// Celsius to Kelvin.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_celsius_to_kelvin(c: f64) -> f64 { c + 273.15 }

/// Kelvin to Celsius.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kelvin_to_celsius(k: f64) -> f64 { k - 273.15 }

/// Electron-volts to Joules.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ev_to_joules(ev: f64) -> f64 { ev * ELEMENTARY_CHARGE }

/// Joules to electron-volts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_joules_to_ev(j: f64) -> f64 { j / ELEMENTARY_CHARGE }

/// Degrees to radians.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_deg_to_rad(deg: f64) -> f64 { deg * PI / 180.0 }

/// Radians to degrees.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rad_to_deg(rad: f64) -> f64 { rad * 180.0 / PI }

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinematic_v() {
        assert_eq!(unsafe { vitalis_kinematic_v(0.0, 9.81, 3.0) }, 29.43);
    }

    #[test]
    fn test_kinematic_s() {
        let s = unsafe { vitalis_kinematic_s(0.0, 9.81, 3.0) };
        assert!((s - 44.145).abs() < 0.001);
    }

    #[test]
    fn test_kinetic_energy() {
        let ke = unsafe { vitalis_kinetic_energy(2.0, 3.0) };
        assert!((ke - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_pendulum() {
        let t = unsafe { vitalis_pendulum_period(1.0, 9.81) };
        assert!((t - 2.006).abs() < 0.01);
    }

    #[test]
    fn test_ideal_gas() {
        // 1 mol at 273.15K in 0.02241 m³ → ~1 atm ≈ 101325 Pa
        let p = unsafe { vitalis_ideal_gas_pressure(1.0, 273.15, 0.02241) };
        assert!((p - 101325.0).abs() < 500.0);
    }

    #[test]
    fn test_carnot() {
        let eta = unsafe { vitalis_carnot_efficiency(300.0, 600.0) };
        assert!((eta - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_coulomb() {
        let f = unsafe { vitalis_coulomb_force(1e-6, 1e-6, 0.1) };
        assert!((f - 0.8988).abs() < 0.01);
    }

    #[test]
    fn test_photon_energy() {
        // Green light ~5.5e14 Hz
        let e = unsafe { vitalis_photon_energy(5.5e14) };
        assert!((e - 3.644e-19).abs() < 1e-21);
    }

    #[test]
    fn test_snell() {
        // Air to glass: n1=1.0, θ1=π/4, n2=1.5
        let theta2 = unsafe { vitalis_snell(1.0, PI / 4.0, 1.5) };
        assert!((theta2 - 0.4817).abs() < 0.01);
    }

    #[test]
    fn test_radioactive_decay() {
        let lambda = unsafe { vitalis_decay_constant(5730.0) }; // Carbon-14
        let n = unsafe { vitalis_radioactive_decay(1000.0, lambda, 5730.0) };
        assert!((n - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_mass_energy() {
        let e = unsafe { vitalis_mass_energy(1.0) }; // 1 kg → E = c²
        assert!((e - 8.9875e16).abs() < 1e13);
    }

    #[test]
    fn test_ph() {
        let ph = unsafe { vitalis_ph(1e-7) };
        assert!((ph - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_schwarzschild() {
        // Sun mass ~2e30 kg, Rs ≈ 2954 m
        let rs = unsafe { vitalis_schwarzschild_radius(2e30) };
        assert!((rs - 2964.0).abs() < 30.0);
    }

    #[test]
    fn test_reynolds() {
        // Water: ρ=1000, v=1, L=0.1, μ=0.001 → Re=100000
        let re = unsafe { vitalis_reynolds_number(1000.0, 1.0, 0.1, 0.001) };
        assert!((re - 100_000.0).abs() < 1.0);
    }

    #[test]
    fn test_celsius_kelvin() {
        assert!((unsafe { vitalis_celsius_to_kelvin(0.0) } - 273.15).abs() < 1e-10);
        assert!((unsafe { vitalis_kelvin_to_celsius(273.15) }).abs() < 1e-10);
    }

    #[test]
    fn test_deg_rad() {
        let rad = unsafe { vitalis_deg_to_rad(180.0) };
        assert!((rad - PI).abs() < 1e-10);
        let deg = unsafe { vitalis_rad_to_deg(PI) };
        assert!((deg - 180.0).abs() < 1e-10);
    }

    #[test]
    fn test_escape_velocity() {
        // Earth: M=5.97e24, R=6.37e6 → v_esc ≈ 11186 m/s
        let v = unsafe { vitalis_escape_velocity(5.97e24, 6.37e6) };
        assert!((v - 11186.0).abs() < 20.0);
    }
}

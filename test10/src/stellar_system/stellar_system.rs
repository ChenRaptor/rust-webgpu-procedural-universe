use std::f64::consts::PI;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use glam::Vec3;

// Constante
const G: f64 = 6.67430e-11;
const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;
// 1 000                                                10**3
// 1 000 000 kg = 1 kilotonne (kt)                      10**6
// 1 000 000 000 kg = 1 mégatonne (Mt)                  10**9
// 1 000 000 000 000 kg = 1 gigatonne (Gt)              10**12

// 1 000 000 000 000 000 kg = 1 tératonne (Tt)          10**15

// 1 000 000 000 000 000 000 kg = 1 pétatonne (Pt)      10**18

// 1 000 000 000 000 000 000 000 kg = 1 exatonne (Et)   10**21

// zetta  10**24

// yottatonne  10**27


// #[repr(transparent)]
// struct Yt(u32);

// type Et = u32;
// type Zt = u32;
type Yt = f64;






pub struct RNG {
    state: u64,
}

impl RNG {
    pub fn new(pos: Vec3) -> Self {
        // Hash la position pour obtenir une seed
        let mut hasher = DefaultHasher::new();
        pos.x.to_bits().hash(&mut hasher);
        pos.y.to_bits().hash(&mut hasher);
        pos.z.to_bits().hash(&mut hasher);
        let seed = hasher.finish();
        Self { state: seed }
    }

    // Xorshift64*
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    // Xorshift32*
    fn next_u32(&mut self) -> u32 {
        let mut x = self.state as u32;
        x ^= x >> 13;
        x ^= x << 17;
        x ^= x >> 5;
        // Met à jour l'état en utilisant le résultat pour garder la séquence pseudo-aléatoire
        self.state = (self.state & 0xFFFFFFFF00000000) | (x as u64);
        x.wrapping_mul(0x85ebca6b)
    }

    pub fn u32(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next_u32() % (max - min))
    }

    pub fn u64(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next_u64() % (max - min))
    }


    pub fn gen_norm(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    pub fn f64(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.gen_norm()
    }

}

pub enum CelestialBody {
    Star(Star),
    Planet(Planet)
}

pub struct Star {
    pub name: String,
    pub physical_props: StarPhysicalProperties,
    pub position: glam::Vec3,
    pub velocity: glam::Vec3,
}

pub struct Planet {
    pub name: String,
    pub physical_props: PlanetPhysicalProperties,
    pub position: glam::Vec3,
    pub velocity: glam::Vec3,
}

pub enum StellarSystemType {
    Single,
    Binary,
    Ternary
}

pub struct StellarSystem {
    pub name: String,
    pub system_type: StellarSystemType,
    pub bodies: Vec<CelestialBody>,
}

#[derive(Debug, Clone)]
pub struct StarPhysicalProperties {
    pub mass: f64,         // en masses solaires
    pub luminosity: f64,   // en luminosités solaires
    pub radius: f64,       // en rayons solaires
    pub temperature: f64,  // en Kelvin
    pub lifetime: f64,     // en milliards d'années
    pub spectral_type: String,
}

#[derive(Debug, Clone)]
pub struct PlanetPhysicalProperties {
    pub mass: f64,   // en masses terrestres
    pub radius: f64, // en rayons terrestres
    // Vous pouvez ajouter d'autres propriétés (densité, type, etc.)
}

#[derive(Debug, Clone)]
pub struct StarComposition {
    pub hydrogen: f32,
    pub helium: f32,
    pub metals: f32,
}

/// Calcule le rayon d'une étoile de la séquence principale en fonction de la masse (en Yt) et de la métallicité (composition.metals)
pub fn star_radius_from_mass_composition(mass: f64, composition: &StarComposition) -> f64 {
    // Conversion Yt -> masses solaires
    let mass_solar = mass / 2000.0;
    // Influence de la métallicité : plus de métaux = rayon légèrement plus petit
    // (effet empirique, typiquement -5% à -10% pour Z=0.03 vs Z=0.001)
    let metallicity_factor = 1.0 - 0.5 * composition.metals as f64; // 0.5 = force de l'effet
    let base_radius = mass_solar.powf(0.8); // relation séquence principale
    base_radius * metallicity_factor
}

pub fn surface_temperature_from_mass_radius_composition(mass_yt: f64, radius_solar: f64, metallicity: f64) -> f64 {
    let mass_solar = mass_yt / 2000.0;
    let base_temp = 5778.0 * (mass_solar / radius_solar.powi(2)).powf(0.25);
    let metallicity_factor = 1.0 - 0.1 * (metallicity / 0.0134 - 1.0); // Z_sun = 0.0134
    base_temp * metallicity_factor
}

pub fn compute_luminosity(radius: f64, temperature: f64) -> f64
{
    4.0 * PI * radius.powf(2.0) * STEFAN_BOLTZMANN * temperature.powf(4.0)
}


/// Génère une composition stellaire en fonction de plusieurs facteurs physiques et galactiques.
/// Les poids sont ajustables pour chaque facteur.
pub fn generate_star_composition(
    mass: f64,
    age: f64,                 // en milliards d'années (0 = très jeune, 13.7 = très vieux)
    supernovae_proximity: f64,// 0.0 (loin) à 1.0 (très proche)
    galactic_history: f64,    // 0.0 (peu de générations) à 1.0 (beaucoup)
    galactic_region: f64      // 0.0 (halo/périphérie) à 1.0 (centre/bras spiraux)
) -> StarComposition {
    // Poids de chaque facteur (somme = 1.0)
    let w_age = 0.25;
    let w_supernovae = 0.20;
    let w_history = 0.25;
    let w_region = 0.20;
    let w_mass = 0.10;

    // Métallicité de base (Population II)




    let mut metals: f64 = 0.0005;
    // Influence de chaque facteur (valeurs empiriques)
    metals += w_age * (1.0 - age / 13.7) * 0.02; // plus jeune = plus de métaux
    metals += w_supernovae * supernovae_proximity * 0.02;
    metals += w_history * galactic_history * 0.02;
    metals += w_region * galactic_region * 0.02;
    metals += w_mass * (mass / 200000.0).min(1.0) * 0.01; // étoiles massives légèrement plus riches
    metals = metals.clamp(0.0001, 0.04); // bornes réalistes

    // Hélium augmente légèrement avec la métallicité
    let helium = 0.24 + metals * 1.5;
    // Hydrogène = reste
    let hydrogen = 1.0 - helium - metals;

    StarComposition {
        hydrogen: hydrogen as f32,
        helium: helium as f32,
        metals: metals as f32,
    }
}

pub fn generate_star(rng: &mut RNG) -> StarPhysicalProperties {

    // Masse entre 160 et 200 000 Yt
    let mass: Yt = rng.f64(160.0,200000.0);
    let composition = generate_star_composition(mass, 6.2, 0.0, 0.0, 0.0);
    let radius = star_radius_from_mass_composition(mass, &composition);
    let temperature = surface_temperature_from_mass_radius_composition(mass, radius, 1.0 - 0.5 * composition.metals as f64);
    let luminosity = compute_luminosity(radius, temperature);
    let lifetime = 10.0 * (mass / luminosity); // en milliards d'années
    let spectral_type = match temperature as u32 {
        t if t >= 30000 => "O",
        t if t >= 10000 => "B",
        t if t >= 7500  => "A",
        t if t >= 6000  => "F",
        t if t >= 5200  => "G",
        t if t >= 3700  => "K",
        _              => "M",
    }.to_string();
    StarPhysicalProperties {
        mass,
        luminosity,
        radius,
        temperature,
        lifetime,
        spectral_type,
    }
}

pub fn generate_planet(rng: &mut RNG) -> PlanetPhysicalProperties {
    // Masse entre 0.1 et 3000 masses terrestres (de Mercure à Jupiter)
    let mass = rng.f64(0.1, 3000.0);
    // Rayon selon la masse (approximation simplifiée)
    // Pour les planètes telluriques (M < 10) : R ~ M^0.3
    // Pour les géantes gazeuses (M >= 10) : R ~ M^0.5 (saturé vers 11 R_terre)
    let radius = if mass < 10.0 {
        mass.powf(0.3)
    } else {
        (mass.powf(0.5)).min(11.0)
    };
    PlanetPhysicalProperties {
        mass,
        radius: 1.0,
    }
}

impl StellarSystem {
    pub fn new(pos: Vec3) -> StellarSystem {
        let mut rng = RNG::new(pos);



        let num_stars: u32 = Self::get_number_star(&mut rng, 0.5, 3);
        let mut bodies = Vec::new();
        let mut barycenter = Vec3::ZERO;
        let mut total_mass = 0.0;

        for i in 0..num_stars {
            // let mass = rng.gen_float(1.0e30, 2.0e30);
            let star_props = generate_star(&mut rng);
            let distance = if i == 0 { 0.0 } else { rng.f64(1.0e10, 1.5e11) as f32 };
            let position = Vec3::new(distance / 1.0e10, 0.0, 0.0);


            barycenter = (barycenter * total_mass as f32 + position * star_props.mass as f32) / (total_mass + star_props.mass) as f32;
            total_mass += star_props.mass;

            let mut star = Star {
                name: format!("Star {}", i + 1),
                physical_props: star_props,
                position,
                velocity: Vec3::ZERO,
            };

            let r_vec = star.position - barycenter;
            let r = r_vec.length() as f64;
            if r > 0.0 {
                let v_mag = (G * total_mass / r).sqrt() as f32;
                let dir = Vec3::new(-r_vec.y, r_vec.x, 0.0).normalize();
                star.velocity = dir * v_mag;
            }

            bodies.push(CelestialBody::Star(star));
        }

        let num_planets: u32 = Self::get_number_star(&mut rng, 0.4, 8);

        for i in 0..num_planets {

            let planet_props = generate_planet(&mut rng);
            let distance = rng.f64(5.0e10, 5.0e11) as f32;
            let angle = rng.f64(0.0, std::f32::consts::TAU as f64) as f32;
            let pos = barycenter + Vec3::new((distance / 1.0e10) * angle.cos(), (distance / 1.0e10) * angle.sin(), 0.0);

            let r_vec = pos - barycenter;
            let v_mag = (G * total_mass / r_vec.length() as f64).sqrt() as f32;
            let dir = Vec3::new(-r_vec.y, r_vec.x, 0.0).normalize();

            let planet = Planet {
                name: format!("Planet {}", i + 1),
                physical_props: planet_props,
                position: pos,
                velocity: dir * v_mag,
            };

            bodies.push(CelestialBody::Planet(planet));
        }

        StellarSystem {
            name: String::from("dzdzd"),
            system_type: StellarSystemType::Binary,
            bodies,
        }
    }

    fn get_number_star(rng: &mut RNG, p: f64, max_stars: u32) -> u32 {
        let mut n: u32 = 1;
        while n < max_stars && rng.gen_norm() < p {
            n += 1;
        }
        n
    }
}

// Exemple d'utilisation :
// let radius = star_radius_from_mass_composition(mass, &composition);


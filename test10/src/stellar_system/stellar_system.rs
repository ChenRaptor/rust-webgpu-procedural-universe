use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use glam::Vec3; // pour les coordonnées

const G: f64 = 6.67430e-11;

struct RNG {
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

    pub fn gen_norm(&mut self) -> f64 {
        // [0,1)
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    pub fn gen_float(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.gen_norm()
    }

    pub fn gen_uint(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next_u64() % (max - min))
    }
}

pub enum CelestialBody {
    Star(Star),
    Planet(Planet)
}

pub struct Star {
    pub name: String,
    pub mass: f64,
    pub radius: f32,
    pub position: glam::Vec3,
    pub velocity: glam::Vec3,
}

pub struct Planet {
    pub name: String,
    pub mass: f64,
    pub radius: f32,
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

impl StellarSystem {
    pub fn new(pos: Vec3) -> StellarSystem {
        let mut rng = RNG::new(pos);

        let num_stars: u32 = Self::get_number_star(&mut rng, 0.5, 3);
        let mut bodies = Vec::new();
        let mut barycenter = Vec3::ZERO;
        let mut total_mass = 0.0;

        for i in 0..num_stars {
            let mass = rng.gen_float(1.0e30, 2.0e30);
            let distance = if i == 0 { 0.0 } else { rng.gen_float(1.0e10, 1.5e11) as f32 };
            let position = Vec3::new(distance / 1.0e10, 0.0, 0.0);

            barycenter = (barycenter * total_mass as f32 + position * mass as f32) / (total_mass + mass) as f32;
            total_mass += mass;

            let mut star = Star {
                name: format!("Star {}", i + 1),
                mass,
                radius: 1.0,
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
            let distance = rng.gen_float(5.0e10, 5.0e11) as f32;
            let angle = rng.gen_float(0.0, std::f32::consts::TAU as f64) as f32;
            let pos = barycenter + Vec3::new((distance / 1.0e10) * angle.cos(), (distance / 1.0e10) * angle.sin(), 0.0);

            let r_vec = pos - barycenter;
            let v_mag = (G * total_mass / r_vec.length() as f64).sqrt() as f32;
            let dir = Vec3::new(-r_vec.y, r_vec.x, 0.0).normalize();

            let planet = Planet {
                name: format!("Planet {}", i + 1),
                mass: rng.gen_float(1e24,1e27),
                radius: 1.0,
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


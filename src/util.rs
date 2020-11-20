use ultraviolet::{Vec3, Rotor3};
use rand::Rng;

// http://corysimon.github.io/articles/uniformdistn-on-sphere/
// I copied this function from a previous project I was working on a while ago.
// I think technically the x and y arguments should be switched because we're using
// a Y-up coordinate system but whatever.
pub fn uniform_sphere_distribution_from_coords(x: f64, y: f64) -> Vec3 {
    use std::f64::consts::PI;

    let theta = 2.0 * PI * x;
    let phi = (1.0 - 2.0 * y).acos();

    Vec3::new(
        (phi.sin() * theta.cos()) as f32,
        (phi.sin() * theta.sin()) as f32,
        phi.cos() as f32,
    )
}

pub fn random_rotation<R: Rng>(rng: &mut R) -> Rotor3 {
    Rotor3::from_rotation_between(
        Vec3::new(0.0, 1.0, 0.0),
        uniform_sphere_distribution_from_coords(rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0))
    )
}

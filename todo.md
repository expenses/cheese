- Add health bars (important)
- Better selection visibility (medium)
- Mouse modeling, texturing rigging and animating (important, can save for later)
- Add animation system to renderer (medium)
- Fill out HUD (medium)
- Fancy pants torus renderer that draws tori of different radii
struct Instance {
  position: Vec3,
  radius: f32,
}
in the vertex shader
 - takes vertex position
 - sets the y to zero
 - takes the magnitude, adds (radius - 1)
 - adds the vertex * modified magnitude to the position

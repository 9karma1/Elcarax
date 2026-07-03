struct VertexIn {
  @location(0) position: vec2<f32>,
  @location(1) rect: vec4<f32>,
  @location(2) color: vec4<f32>,
  @location(3) size_px: vec2<f32>,
  @location(4) radius_px: vec4<f32>,
};
struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) local: vec2<f32>,
  @location(2) size_px: vec2<f32>,
  @location(3) radius_px: vec4<f32>,
};
@vertex
fn vs_main(input: VertexIn) -> VertexOut {
  var out: VertexOut;
  let pos = input.rect.xy + input.position * input.rect.zw;
  out.position = vec4<f32>(pos, 0.0, 1.0);
  out.color = input.color;
  out.local = input.position;
  out.size_px = input.size_px;
  out.radius_px = input.radius_px;
  return out;
}

// radius_px is ordered (top_left, top_right, bottom_right, bottom_left), matching
// the CornerRadius struct on the Rust side. Picks the radius for the quadrant
// that contains p, where p is relative to the rect center.
fn corner_radius(p: vec2<f32>, radius_px: vec4<f32>) -> f32 {
  let is_top = p.y < 0.0;
  let is_left = p.x < 0.0;
  let top_radius = select(radius_px.y, radius_px.x, is_left);
  let bottom_radius = select(radius_px.z, radius_px.w, is_left);
  return select(bottom_radius, top_radius, is_top);
}

// Signed distance from p to a rounded box edge, in the same units as
// half_size/radius. Negative inside the box, positive outside.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
  let clamped_radius = min(radius, min(half_size.x, half_size.y));
  let q = abs(p) - half_size + vec2<f32>(clamped_radius, clamped_radius);
  return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - clamped_radius;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
  let total_radius =
    input.radius_px.x + input.radius_px.y + input.radius_px.z + input.radius_px.w;
  if total_radius <= 0.0001 {
    // Sharp rectangle: skip the distance field entirely so pixel-perfect
    // primitives (glyphs, lines, border strips, flat fills) render unchanged.
    return input.color;
  }
  let half_size = input.size_px * 0.5;
  let p = (input.local - vec2<f32>(0.5, 0.5)) * input.size_px;
  let radius = corner_radius(p, input.radius_px);
  let distance = sd_rounded_box(p, half_size, radius);
  let edge = max(fwidth(distance) * 0.5, 0.0001);
  let coverage = 1.0 - smoothstep(-edge, edge, distance);
  return vec4<f32>(input.color.rgb, input.color.a * coverage);
}

struct VertexIn {
  @location(0) position: vec2<f32>,
  @location(1) rect: vec4<f32>,
  @location(2) uv_rect: vec4<f32>,
  @location(3) opacity: f32,
};
struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) opacity: f32,
};
@group(0) @binding(0) var viewport_sampler: sampler;
@group(0) @binding(1) var viewport_texture: texture_2d<f32>;
@vertex
fn vs_main(input: VertexIn) -> VertexOut {
  var out: VertexOut;
  let pos = input.rect.xy + input.position * input.rect.zw;
  out.position = vec4<f32>(pos, 0.0, 1.0);
  out.uv = input.uv_rect.xy + input.position * input.uv_rect.zw;
  out.opacity = input.opacity;
  return out;
}
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
  let color = textureSample(viewport_texture, viewport_sampler, input.uv);
  return vec4<f32>(color.rgb, color.a * input.opacity);
}

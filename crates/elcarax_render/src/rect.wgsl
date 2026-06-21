struct VertexIn {
  @location(0) position: vec2<f32>,
  @location(1) rect: vec4<f32>,
  @location(2) color: vec4<f32>,
};
struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
};
@vertex
fn vs_main(input: VertexIn) -> VertexOut {
  var out: VertexOut;
  let pos = input.rect.xy + input.position * input.rect.zw;
  out.position = vec4<f32>(pos, 0.0, 1.0);
  out.color = input.color;
  return out;
}
@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
  return input.color;
}

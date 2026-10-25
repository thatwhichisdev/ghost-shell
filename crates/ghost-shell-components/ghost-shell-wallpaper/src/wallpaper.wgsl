@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@group(0) @binding(2) var<uniform> crop: vec4<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let position = positions[index];
    var result: Vertex;
    result.position = vec4(position, 0.0, 1.0);
    let uv = vec2((position.x + 1.0) * 0.5, (1.0 - position.y) * 0.5);
    result.uv = (uv - vec2(0.5)) * crop.xy + vec2(0.5);
    return result;
}
@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, input.uv);
}

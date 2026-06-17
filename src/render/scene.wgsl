struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct Camera {
    matrix: mat4x4<f32>,
};

struct Surface {
    origin: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;
@group(2) @binding(0) var<uniform> surface: Surface;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.matrix * vec4<f32>(input.position + surface.origin.xyz, 1.0);
    output.color = input.color;
    output.tex_coord = input.tex_coord;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(atlas, atlas_sampler, input.tex_coord).r;
    // return vec4<f32>(input.color.rgb, input.color.a * sample); // non-sdf path

    let distance = sample;
    let width = max(fwidth(distance) * 1.5, 0.004);
    let alpha = smoothstep(0.5 - width, 0.5 + width, distance);
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}

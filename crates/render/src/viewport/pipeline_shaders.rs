use std::borrow::Cow;

use egui_wgpu::wgpu;

const MAIN_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    key_dir: vec3<f32>,
    _pad0: f32,
    fill_dir: vec3<f32>,
    _pad1: f32,
    rim_dir: vec3<f32>,
    _pad2: f32,
    camera_pos: vec3<f32>,
    _pad3: f32,
    base_color: vec3<f32>,
    _pad4: f32,
    light_params: vec4<f32>,
    debug_params: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var shadow_tex: texture_depth_2d;

@group(0) @binding(2)
var shadow_sampler: sampler_comparison;

struct Material {
    base_color: vec4<f32>,
    params: vec4<f32>,
};

@group(1) @binding(0)
var<storage, read> materials: array<Material>;

@group(1) @binding(1)
var material_sampler: sampler;

@group(1) @binding(2)
var material_texture: texture_2d_array<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) material: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) material: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = input.position;
    out.normal = input.normal;
    out.color = input.color;
    out.uv = input.uv;
    out.material = input.material;
    out.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    return out;
}

fn shadow_factor(world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    let enabled = select(0.0, 1.0, uniforms.shadow_params.x >= 0.5);
    let n = normalize(normal);
    let light_dir = normalize(uniforms.key_dir);
    let ndotl = max(dot(n, light_dir), 0.0);
    let bias = uniforms.shadow_params.y + (1.0 - ndotl) * uniforms.shadow_params.y;
    let offset_pos = world_pos + n * uniforms.shadow_params.w;
    let light_clip = uniforms.light_view_proj * vec4<f32>(offset_pos, 1.0);
    let ndc = light_clip.xyz / max(light_clip.w, 0.0001);
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let in_bounds = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && ndc.z >= 0.0 && ndc.z <= 1.0;
    let in_bounds_f = select(0.0, 1.0, in_bounds);
    let uv_clamped = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let depth = ndc.z - bias;
    let texel = max(uniforms.shadow_params.z, 0.000001);
    var shadow = 0.0;
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(-texel, -texel), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(0.0, -texel), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(texel, -texel), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(-texel, 0.0), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped, depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(texel, 0.0), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(-texel, texel), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(0.0, texel), depth);
    shadow = shadow + textureSampleCompare(shadow_tex, shadow_sampler, uv_clamped + vec2<f32>(texel, texel), depth);
    let visibility = shadow / 9.0;
    let lit = 1.0 - in_bounds_f + visibility * in_bounds_f;
    return 1.0 - enabled + enabled * lit;
}

fn shade_surface(
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    color: vec3<f32>,
    metallic: f32,
    roughness: f32,
) -> vec3<f32> {
    let n = normalize(normal);
    let view_dir = normalize(uniforms.camera_pos - world_pos);
    let key_dir = normalize(uniforms.key_dir);
    let fill_dir = normalize(uniforms.fill_dir);
    let rim_dir = normalize(uniforms.rim_dir);

    let key_ndotl = max(dot(n, key_dir), 0.0);
    let fill_ndotl = max(dot(n, fill_dir), 0.0);
    let rim_ndotl = max(dot(n, rim_dir), 0.0);

    let half_dir = normalize(key_dir + view_dir);
    let shininess = mix(64.0, 8.0, clamp(roughness, 0.0, 1.0));
    let spec = pow(max(dot(n, half_dir), 0.0), shininess);

    let shadow = shadow_factor(world_pos, normal);
    let key = key_ndotl * uniforms.light_params.x * shadow;
    let fill = fill_ndotl * uniforms.light_params.y;
    let rim = rim_ndotl * uniforms.light_params.z;
    let ambient = uniforms.light_params.w;

    let base = color * uniforms.base_color;
    let metallic_factor = clamp(metallic, 0.0, 1.0);
    let diffuse = base * (1.0 - metallic_factor);
    let specular_strength = mix(0.04, 1.0, metallic_factor);
    return diffuse * (ambient + key + fill + rim) + vec3<f32>(0.9) * spec * specular_strength * shadow;
}

fn material_albedo(material_id: u32, uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let mat = materials[material_id];
    var albedo = color * mat.base_color.xyz;
    let tex_index = i32(mat.params.y);
    if tex_index >= 0 {
        let uv_scaled = uv * mat.params.zw;
        let tex = textureSample(material_texture, material_sampler, uv_scaled, tex_index).rgb;
        albedo = albedo * tex;
    }
    return albedo;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let mat = materials[input.material];
    let albedo = material_albedo(input.material, input.uv, input.color);
    let color = shade_surface(input.normal, input.world_pos, albedo, mat.base_color.w, mat.params.x);
    let mode = i32(uniforms.debug_params.x + 0.5);
    if mode == 1 {
        let normal = normalize(input.normal);
        return vec4<f32>(normal * 0.5 + vec3<f32>(0.5), 1.0);
    }
    if mode == 2 {
        let near = uniforms.debug_params.y;
        let far = uniforms.debug_params.z;
        let denom = max(far - near, 0.0001);
        let dist = distance(uniforms.camera_pos, input.world_pos);
        let t = clamp((dist - near) / denom, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(1.0 - t), 1.0);
    }
    return vec4<f32>(color, 1.0);
}

struct ShadowOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_shadow(input: VertexInput) -> ShadowOutput {
    var out: ShadowOutput;
    out.position = uniforms.light_view_proj * vec4<f32>(input.position, 1.0);
    return out;
}

struct LineInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct LineOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_line(input: LineInput) -> LineOutput {
    var out: LineOutput;
    out.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_line(input: LineOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}

struct SplatInput {
    @location(0) center: vec3<f32>,
    @location(1) offset: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) scale: f32,
};

struct SplatOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) center: vec3<f32>,
    @location(3) scale: f32,
};

@vertex
fn vs_splat(input: SplatInput) -> SplatOutput {
    var out: SplatOutput;
    let clip = uniforms.view_proj * vec4<f32>(input.center, 1.0);
    out.position = clip + vec4<f32>(input.offset * clip.w, 0.0, 0.0);
    out.uv = input.uv;
    out.color = input.color;
    out.center = input.center;
    out.scale = input.scale;
    return out;
}

@fragment
fn fs_splat(input: SplatOutput) -> @location(0) vec4<f32> {
    let r2 = dot(input.uv, input.uv);
    let weight = exp(-0.5 * r2);
    let mode = i32(uniforms.debug_params.x + 0.5);
    if mode == 2 {
        let near = uniforms.debug_params.y;
        let far = uniforms.debug_params.z;
        let denom = max(far - near, 0.0001);
        let dist = distance(uniforms.camera_pos, input.center);
        let t = clamp((dist - near) / denom, 0.0, 1.0);
        let shade = 1.0 - t;
        return vec4<f32>(vec3<f32>(shade), weight);
    }
    if mode == 3 {
        let min_v = uniforms.debug_params.y;
        let max_v = uniforms.debug_params.z;
        let denom = max(max_v - min_v, 0.0001);
        let opacity = clamp(input.color.a, 0.0, 1.0);
        let t = clamp((opacity - min_v) / denom, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(t), weight);
    }
    if mode == 4 {
        let min_v = uniforms.debug_params.y;
        let max_v = uniforms.debug_params.z;
        let denom = max(max_v - min_v, 0.0001);
        let t = clamp((input.scale - min_v) / denom, 0.0, 1.0);
        let color = vec3<f32>(t, 0.2, 1.0 - t);
        return vec4<f32>(color, weight);
    }
    if mode == 5 {
        let min_v = uniforms.debug_params.y;
        let max_v = uniforms.debug_params.z;
        let denom = max(max_v - min_v, 0.0001);
        let t = clamp((weight - min_v) / denom, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(t), weight);
    }
    let alpha = input.color.a * weight;
    let rgb = input.color.rgb;
    return vec4<f32>(rgb, alpha);
}
"#;

const BLIT_WGSL: &str = r#"
@group(0) @binding(0)
var blit_tex: texture_2d<f32>;

@group(0) @binding(1)
var blit_sampler: sampler;

struct BlitOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) index: u32) -> BlitOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: BlitOut;
    out.position = vec4<f32>(positions[index], 0.0, 1.0);
    out.uv = uvs[index];
    return out;
}

@fragment
fn fs_blit(input: BlitOut) -> @location(0) vec4<f32> {
    return textureSample(blit_tex, blit_sampler, input.uv);
}
"#;

pub(super) fn create_main_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lobedo_viewport_shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(MAIN_WGSL)),
    })
}

pub(super) fn create_blit_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lobedo_viewport_blit"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BLIT_WGSL)),
    })
}

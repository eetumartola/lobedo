use std::borrow::Cow;

use egui_wgpu::wgpu;

const MAIN_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
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
    splat_params: vec4<f32>,
    splat_view_x: vec3<f32>,
    _pad5: f32,
    splat_view_y: vec3<f32>,
    _pad6: f32,
    splat_view_z: vec3<f32>,
    _pad7: f32,
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

struct VolumeParams {
    origin: vec3<f32>,
    voxel_size: f32,
    dims: vec3<u32>,
    kind: u32,
    params: vec4<f32>,
    world_to_volume: mat4x4<f32>,
};

@group(2) @binding(0)
var<uniform> volume_params: VolumeParams;

@group(2) @binding(1)
var volume_tex: texture_3d<f32>;

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
    @location(4) @interpolate(flat) material: u32,
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
    let uv_scaled = uv * mat.params.zw;
    let tex_layer = max(tex_index, 0);
    let tex = textureSample(material_texture, material_sampler, uv_scaled, tex_layer).rgb;
    let use_tex = select(0.0, 1.0, tex_index >= 0);
    let tex_mix = mix(vec3<f32>(1.0), tex, use_tex);
    albedo = albedo * tex_mix;
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
    @location(0) corner: vec2<f32>,
    @location(1) center: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) scale: vec3<f32>,
    @location(4) rotation: vec4<f32>,
};

struct SplatOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) center: vec3<f32>,
    @location(3) scale: f32,
};

const SPLAT_BILLBOARD_RADIUS: f32 = 3.0;

fn quat_to_mat3(q: vec4<f32>) -> mat3x3<f32> {
    let x = q.x;
    let y = q.y;
    let z = q.z;
    let w = q.w;
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let yz = y * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;
    return mat3x3<f32>(
        vec3<f32>(1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy)),
        vec3<f32>(2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx)),
        vec3<f32>(2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy)),
    );
}

fn is_finite_f32(v: f32) -> bool {
    return v == v && abs(v) < 1.0e20;
}

fn is_finite_vec4(v: vec4<f32>) -> bool {
    return is_finite_f32(v.x) && is_finite_f32(v.y) && is_finite_f32(v.z) && is_finite_f32(v.w);
}

@vertex
fn vs_splat(input: SplatInput) -> SplatOutput {
    var out: SplatOutput;
    out.color = input.color;
    out.center = input.center;
    let scale_metric = max(max(input.scale.x, input.scale.y), input.scale.z);
    out.scale = select(0.0, scale_metric, is_finite_f32(scale_metric));
    let width = max(uniforms.splat_params.x, 1.0);
    let height = max(uniforms.splat_params.y, 1.0);
    let tan_half = max(tan(uniforms.splat_params.z * 0.5), 1.0e-6);
    let fy = 0.5 * height / tan_half;
    let fx = fy;
    let view_rot = mat3x3<f32>(uniforms.splat_view_x, uniforms.splat_view_y, uniforms.splat_view_z);
    let pos_view = view_rot * (input.center - uniforms.camera_pos);
    let pos_cam = vec3<f32>(pos_view.x, pos_view.y, -pos_view.z);
    let z = pos_cam.z;
    if !(z > uniforms.splat_params.w) || !is_finite_f32(z) {
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        out.uv = vec2<f32>(0.0);
        return out;
    }
    var quat = vec4<f32>(input.rotation.y, input.rotation.z, input.rotation.w, input.rotation.x);
    if !is_finite_vec4(quat) {
        quat = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let len2 = dot(quat, quat);
    if len2 < 1.0e-8 {
        quat = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else {
        quat = quat * (1.0 / sqrt(len2));
    }
    let rot = quat_to_mat3(quat);
    let scale = input.scale;
    let cov_local = mat3x3<f32>(
        vec3<f32>(scale.x * scale.x, 0.0, 0.0),
        vec3<f32>(0.0, scale.y * scale.y, 0.0),
        vec3<f32>(0.0, 0.0, scale.z * scale.z),
    );
    let cov_world = rot * cov_local * transpose(rot);
    let cov_view = view_rot * cov_world * transpose(view_rot);
    let flip = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, -1.0),
    );
    let cov_cam = flip * cov_view * flip;
    let inv_z = 1.0 / max(z, 1.0e-6);
    let inv_z2 = inv_z * inv_z;
    let j11 = fx * inv_z;
    let j22 = fy * inv_z;
    let j13 = -fx * pos_cam.x * inv_z2;
    let j23 = -fy * pos_cam.y * inv_z2;
    let r0 = vec3<f32>(j11, 0.0, j13);
    let r1 = vec3<f32>(0.0, j22, j23);
    let cov_r0 = cov_cam * r0;
    let cov_r1 = cov_cam * r1;
    let a = dot(r0, cov_r0);
    let b = dot(r0, cov_r1);
    let c = dot(r1, cov_r1);
    let trace = a + c;
    let delta = sqrt(max((a - c) * (a - c) + 4.0 * b * b, 0.0));
    let lambda1 = max(0.5 * (trace + delta), 0.0);
    let lambda2 = max(0.5 * (trace - delta), 0.0);
    let sigma1 = sqrt(lambda1);
    let sigma2 = sqrt(lambda2);
    if sigma1 <= 0.0
        || sigma2 <= 0.0
        || !is_finite_f32(sigma1)
        || !is_finite_f32(sigma2)
    {
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        out.uv = vec2<f32>(0.0);
        return out;
    }
    var v1: vec2<f32>;
    if abs(b) > 1.0e-6 {
        v1 = normalize(vec2<f32>(lambda1 - c, b));
    } else if a >= c {
        v1 = vec2<f32>(1.0, 0.0);
    } else {
        v1 = vec2<f32>(0.0, 1.0);
    }
    if dot(v1, v1) < 1.0e-6 {
        v1 = vec2<f32>(1.0, 0.0);
    }
    let v2 = vec2<f32>(-v1.y, v1.x);
    let axis1 = v1 * (sigma1 * SPLAT_BILLBOARD_RADIUS);
    let axis2 = v2 * (sigma2 * SPLAT_BILLBOARD_RADIUS);
    let axis1_ndc = vec2<f32>(axis1.x * 2.0 / width, axis1.y * 2.0 / height);
    let axis2_ndc = vec2<f32>(axis2.x * 2.0 / width, axis2.y * 2.0 / height);
    let clip = uniforms.view_proj * vec4<f32>(input.center, 1.0);
    let ndc_offset = axis1_ndc * input.corner.x + axis2_ndc * input.corner.y;
    out.position = clip + vec4<f32>(ndc_offset * clip.w, 0.0, 0.0);
    out.uv = input.corner * SPLAT_BILLBOARD_RADIUS;
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

struct VolumeOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_volume(@builtin(vertex_index) index: u32) -> VolumeOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0),
    );
    var out: VolumeOut;
    out.position = vec4<f32>(positions[index], 0.0, 1.0);
    out.uv = uvs[index];
    return out;
}

fn intersect_aabb(origin: vec3<f32>, dir: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec2<f32> {
    let inv_dir = (1.0 / max(abs(dir), vec3<f32>(1.0e-6))) * sign(dir);
    let t0 = (bmin - origin) * inv_dir;
    let t1 = (bmax - origin) * inv_dir;
    let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return vec2<f32>(tmin, tmax);
}

fn sample_volume_density(local_pos: vec3<f32>) -> f32 {
    let dims = volume_params.dims;
    if dims.x == 0u || dims.y == 0u || dims.z == 0u {
        return 0.0;
    }
    let voxel = max(volume_params.voxel_size, 1.0e-6);
    let grid = (local_pos - volume_params.origin) / voxel;
    let ix = i32(floor(grid.x));
    let iy = i32(floor(grid.y));
    let iz = i32(floor(grid.z));
    if ix < 0 || iy < 0 || iz < 0 {
        return 0.0;
    }
    if ix >= i32(dims.x) || iy >= i32(dims.y) || iz >= i32(dims.z) {
        return 0.0;
    }
    let value = textureLoad(volume_tex, vec3<i32>(ix, iy, iz), 0).x;
    let scale = volume_params.params.x;
    if volume_params.kind == 0u {
        return max(value, 0.0) * scale;
    }
    let band = max(volume_params.params.y, 1.0e-6);
    let d = abs(value);
    return exp(-d / band) * scale;
}

@fragment
fn fs_volume(input: VolumeOut) -> @location(0) vec4<f32> {
    let dims = volume_params.dims;
    if dims.x == 0u || dims.y == 0u || dims.z == 0u {
        return vec4<f32>(0.0);
    }
    let ndc = vec4<f32>(input.uv * 2.0 - vec2<f32>(1.0), 1.0, 1.0);
    let world_far = uniforms.inv_view_proj * ndc;
    let world_pos = world_far.xyz / max(world_far.w, 1.0e-6);
    let ray_dir = normalize(world_pos - uniforms.camera_pos);
    let local_origin = (volume_params.world_to_volume * vec4<f32>(uniforms.camera_pos, 1.0)).xyz;
    let local_dir = normalize((volume_params.world_to_volume * vec4<f32>(ray_dir, 0.0)).xyz);
    let dims_f = vec3<f32>(f32(dims.x), f32(dims.y), f32(dims.z));
    let bmin = volume_params.origin;
    let bmax = volume_params.origin + dims_f * volume_params.voxel_size;
    let hit = intersect_aabb(local_origin, local_dir, bmin, bmax);
    let tmin = hit.x;
    let tmax = hit.y;
    if tmax <= tmin {
        return vec4<f32>(0.0);
    }
    let step = max(volume_params.voxel_size, 1.0e-6);
    var t = max(tmin, 0.0);
    let t_end = tmax;
    var accum_color = vec3<f32>(0.0);
    var accum_alpha = 0.0;
    var steps = 0u;
    loop {
        if t > t_end || accum_alpha > 0.98 || steps >= 512u {
            break;
        }
        let pos = local_origin + local_dir * t;
        let density = sample_volume_density(pos);
        if density > 0.0 {
            let alpha = 1.0 - exp(-density * step);
            let contrib = (1.0 - accum_alpha) * alpha;
            accum_color = accum_color + uniforms.base_color * contrib;
            accum_alpha = accum_alpha + contrib;
        }
        t = t + step;
        steps = steps + 1u;
    }
    return vec4<f32>(accum_color, accum_alpha);
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

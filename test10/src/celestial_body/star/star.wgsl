// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;


// Matrice de rotation
struct Model {
    model : mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> rotation : Model;

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};


struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) color : vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(2) ndc_pos: vec2<f32>,
    @location(3) star_center_ndc: vec2<f32>,
    @location(4) star_center_w: f32,
    @location(5) static_pos: vec3<f32>,
};

fn fade(t: f32) -> f32 { return t*t*t*(t*(t*6.0 - 15.0) + 10.0); }
fn lerp(a: f32, b: f32, t: f32) -> f32 { return a + (b - a) * t; }
fn smoothstep_fn(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}
fn tanh_approx(x: f32) -> f32 {
    let e2x = exp(2.0 * x);
    return (e2x - 1.0) / (e2x + 1.0);
}

// Hash/gradient helpers
fn hash3(p: vec3<i32>) -> f32 {
    // Hash rapide déterministe -> [0,1)
    // 2654435761u ne rentre pas dans i32, on prend une valeur proche négative
    var n = p.x * 1103515245 + p.y * 12345 + p.z * 1013904223; // 1013904223 = 0x3C6EF372
    n = (n ^ (n >> 13)) * 1274126177;
    let res = f32((n ^ (n >> 16)) & 0x7fffffff) / f32(0x7fffffff);
    return res;
}

fn grad(hash: f32, x: f32, y: f32, z: f32) -> f32 {
    let h = floor(hash * 16.0);
    let u = select(y, x, (h < 8.0) || (h == 12.0) || (h == 13.0));
    let v = select(z, y, (h < 4.0) || (h == 12.0) || (h == 13.0));
    let s1 = select(-u, u, fract(h/2.0) < 0.5);
    let s2 = select(-v, v, fract(h/4.0) < 0.5);
    return s1 + s2; // approx gradient dot
}

fn perlin3(p: vec3<f32>) -> f32 {
    let pi = vec3<i32>(floor(p));
    let pf = p - floor(p);

    let h000 = hash3(pi + vec3<i32>(0,0,0));
    let h100 = hash3(pi + vec3<i32>(1,0,0));
    let h010 = hash3(pi + vec3<i32>(0,1,0));
    let h110 = hash3(pi + vec3<i32>(1,1,0));
    let h001 = hash3(pi + vec3<i32>(0,0,1));
    let h101 = hash3(pi + vec3<i32>(1,0,1));
    let h011 = hash3(pi + vec3<i32>(0,1,1));
    let h111 = hash3(pi + vec3<i32>(1,1,1));

    let u = vec3<f32>(fade(pf.x), fade(pf.y), fade(pf.z));

    let x00 = lerp(grad(h000, pf.x, pf.y, pf.z), grad(h100, pf.x-1.0, pf.y, pf.z), u.x);
    let x10 = lerp(grad(h010, pf.x, pf.y-1.0, pf.z), grad(h110, pf.x-1.0, pf.y-1.0, pf.z), u.x);
    let x01 = lerp(grad(h001, pf.x, pf.y, pf.z-1.0), grad(h101, pf.x-1.0, pf.y, pf.z-1.0), u.x);
    let x11 = lerp(grad(h011, pf.x, pf.y-1.0, pf.z-1.0), grad(h111, pf.x-1.0, pf.y-1.0, pf.z-1.0), u.x);

    let y0 = lerp(x00, x10, u.y);
    let y1 = lerp(x01, x11, u.y);
    // Remap to [-1,1]
    return lerp(y0, y1, u.z) * 2.0 - 1.0;
}

fn fbm_perlin_noise(px: f32, py: f32, pz: f32, octaves: u32, persistence: f32, scale: f32) -> f32 {
    var amp = 1.0;
    var freq = scale;
    var sum = 0.0;
    var norm = 0.0;
    let p = vec3<f32>(px, py, pz);
    for (var i: u32 = 0u; i < octaves; i = i + 1u) {
        let n = perlin3(p * freq);
        sum += n * amp;
        norm += amp;
        amp *= persistence;
        freq *= 2.0;
    }
    return sum / max(norm, 1e-6);
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.color = model.color;
    let clip_position = camera.view_proj * model_matrix * rotation.model * vec4<f32>(model.position, 1.0);
    out.clip_position = clip_position;
    let aspect = 1020.0 / 1305.0;
    out.ndc_pos = vec2<f32>(
        clip_position.x / clip_position.w,
        (clip_position.y / clip_position.w) / aspect
    );
    // Calcul du centre de l'étoile en NDC (on suppose que le centre est la position (0,0,0) dans le modèle)
    let star_center_clip = camera.view_proj * model_matrix * rotation.model * vec4<f32>(0.0, 0.0, 0.0, 1.0);
    out.star_center_ndc = vec2<f32>(
        star_center_clip.x / star_center_clip.w,
        (star_center_clip.y / star_center_clip.w) / aspect
    );
    out.star_center_w = star_center_clip.w;
    out.static_pos = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var noise = fbm_perlin_noise(in.static_pos.x, in.static_pos.y, in.static_pos.z, 4, 0.7, 10.0);
    noise = (noise + 1.0) / 2.0 + 1.1;
    noise = pow(noise, 1.9); // augmente le contraste

    let dist = length(in.ndc_pos - in.star_center_ndc) * in.star_center_w * 0.095;
    let glow = pow(1.0 - smoothstep(0.2, 0.4, dist), 2.0);
    let base_color = vec3<f32>(1.0, 0.8, 0.2);
    let color = base_color * (1.0 + 2.5 * glow) * noise;
    return vec4<f32>(color, 1.0);
}

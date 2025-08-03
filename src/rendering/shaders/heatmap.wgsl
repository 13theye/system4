// Heatmap generation compute shader
// Converts particle positions to smooth heatmap using Gaussian falloff

struct Particle {
    position: vec2<f32>,
    padding: vec2<f32>,
}

struct HeatmapParams {
    resolution: vec2<f32>,
    bounds_min: vec2<f32>,
    bounds_max: vec2<f32>,
    max_influence_radius: f32,
    intensity_scale: f32,
    particle_count: u32,
    padding: u32,
}

@group(0) @binding(0) var heatmap_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<uniform> params: HeatmapParams;

// Thermal/infrared color gradient function
fn color_from_intensity(intensity: f32) -> vec4<f32> {
    let clamped_intensity = clamp(intensity, 0.0, 1.0);
    
    if (clamped_intensity < 0.25) {
        // Dark blue to bright blue
        let t = clamped_intensity / 0.25;
        let dark_blue = vec3<f32>(0.0, 0.0, 0.3);
        let bright_blue = vec3<f32>(0.0, 0.0, 1.0);
        let rgb = dark_blue + (bright_blue - dark_blue) * t;
        return vec4<f32>(rgb, 0.7 + clamped_intensity * 0.2);
    } else if (clamped_intensity < 0.5) {
        // Blue to green
        let t = (clamped_intensity - 0.25) / 0.25;
        let blue = vec3<f32>(0.0, 0.0, 1.0);
        let green = vec3<f32>(0.0, 1.0, 0.0);
        let rgb = blue + (green - blue) * t;
        return vec4<f32>(rgb, 0.8 + clamped_intensity * 0.1);
    } else if (clamped_intensity < 0.75) {
        // Green to yellow
        let t = (clamped_intensity - 0.5) / 0.25;
        let green = vec3<f32>(0.0, 1.0, 0.0);
        let yellow = vec3<f32>(1.0, 1.0, 0.0);
        let rgb = green + (yellow - green) * t;
        return vec4<f32>(rgb, 0.85 + clamped_intensity * 0.1);
    } else {
        // Yellow to red to white
        let t = (clamped_intensity - 0.75) / 0.25;
        let yellow = vec3<f32>(1.0, 1.0, 0.0);
        let red = vec3<f32>(1.0, 0.0, 0.0);
        let white = vec3<f32>(1.0, 1.0, 1.0);
        
        var rgb: vec3<f32>;
        if (t < 0.5) {
            rgb = yellow + (red - yellow) * (t * 2.0);
        } else {
            rgb = red + (white - red) * ((t - 0.5) * 2.0);
        }
        return vec4<f32>(rgb, 0.9);
    }
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coords = vec2<i32>(i32(global_id.x), i32(global_id.y));
    let resolution = vec2<i32>(i32(params.resolution.x), i32(params.resolution.y));
    
    // Check bounds
    if (pixel_coords.x >= resolution.x || pixel_coords.y >= resolution.y) {
        return;
    }
    
    // Convert pixel coordinates to world coordinates (match CPU implementation)
    let bounds_width = params.bounds_max.x - params.bounds_min.x;
    let bounds_height = params.bounds_max.y - params.bounds_min.y;
    let pixel_size_x = bounds_width / params.resolution.x;
    let pixel_size_y = bounds_height / params.resolution.y;
    
    let pixel_world_x = params.bounds_min.x + (f32(pixel_coords.x) + 0.5) * pixel_size_x;
    // Flip Y coordinate to match Nannou's coordinate system
    let flipped_y = params.resolution.y - 1.0 - f32(pixel_coords.y);
    let pixel_world_y = params.bounds_min.y + (flipped_y + 0.5) * pixel_size_y;
    let pixel_pos = vec2<f32>(pixel_world_x, pixel_world_y);
    
    var total_intensity = 0.0;
    let max_influence_radius_sq = params.max_influence_radius * params.max_influence_radius;
    
    // Calculate influence from each particle with optimizations
    for (var i = 0u; i < params.particle_count && i < 1000u; i = i + 1u) { // Limit particles processed
        let particle_pos = particles[i].position;
        
        // Skip invalid particles (marked with large values)
        if (particle_pos.x > 1e30 || particle_pos.y > 1e30) {
            continue;
        }
        
        let dx = pixel_pos.x - particle_pos.x;
        let dy = pixel_pos.y - particle_pos.y;
        let distance_sq = dx * dx + dy * dy;
        
        // Early exit for distant particles
        if (distance_sq > max_influence_radius_sq) {
            continue;
        }
        
        // Gaussian falloff - matches original sigma = 25.0
        let sigma = 25.0;
        let influence = exp(-distance_sq / (2.0 * sigma * sigma));
        total_intensity += influence;
        
        // Early exit if we have enough intensity
        if (total_intensity > 3.0) {
            break;
        }
    }
    
    // Skip pixels with very low intensity
    if (total_intensity < 0.05) {
        textureStore(heatmap_texture, pixel_coords, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }
    
    // Normalize and apply color mapping
    let intensity = clamp(total_intensity * params.intensity_scale, 0.0, 1.0);
    let color = color_from_intensity(intensity);
    
    textureStore(heatmap_texture, pixel_coords, color);
}
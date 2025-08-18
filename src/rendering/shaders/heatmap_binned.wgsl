// Optimized heatmap generation using spatial binning
// Only checks particles in relevant spatial bins instead of all particles

struct Particle {
    position: vec2<f32>,
    padding: vec2<f32>,
}

struct HeatmapParams {
    resolution: vec2<f32>,
    bounds_min: vec2<f32>,
    bounds_max: vec2<f32>,
    grid_size: vec2<u32>,        // Number of spatial bins
    bin_cell_size: vec2<f32>,    // Size of each bin in world units
    max_influence_radius: f32,
    intensity_scale: f32,
    particle_count: u32,
    padding: u32,
}

@group(0) @binding(0) var heatmap_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<storage, read> bin_counts: array<u32>;
@group(0) @binding(3) var<storage, read> bin_data: array<u32>;
@group(0) @binding(4) var<storage, read> bin_offsets: array<u32>;
@group(0) @binding(5) var<uniform> params: HeatmapParams;

// Grayscale gradient function
fn color_from_intensity(intensity: f32) -> vec4<f32> {
    let clamped_intensity = clamp(intensity, 0.0, 1.0);
    let gray_value = 1.0 - clamped_intensity;
    return vec4<f32>(gray_value, gray_value, gray_value, 0.7 + clamped_intensity * 0.2);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coords = vec2<i32>(i32(global_id.x), i32(global_id.y));
    let resolution = vec2<i32>(i32(params.resolution.x), i32(params.resolution.y));
    
    // Check bounds
    if (pixel_coords.x >= resolution.x || pixel_coords.y >= resolution.y) {
        return;
    }
    
    // Convert pixel coordinates to world coordinates
    let bounds_width = params.bounds_max.x - params.bounds_min.x;
    let bounds_height = params.bounds_max.y - params.bounds_min.y;
    let pixel_size_x = bounds_width / params.resolution.x;
    let pixel_size_y = bounds_height / params.resolution.y;
    
    let pixel_world_x = params.bounds_min.x + (f32(pixel_coords.x) + 0.5) * pixel_size_x;
    let flipped_y = params.resolution.y - 1.0 - f32(pixel_coords.y);
    let pixel_world_y = params.bounds_min.y + (flipped_y + 0.5) * pixel_size_y;
    let pixel_pos = vec2<f32>(pixel_world_x, pixel_world_y);
    
    // Calculate which spatial bin this pixel is in
    let normalized_pos = (pixel_pos - params.bounds_min) / (params.bounds_max - params.bounds_min);
    let center_bin_x = i32(normalized_pos.x * f32(params.grid_size.x));
    let center_bin_y = i32(normalized_pos.y * f32(params.grid_size.y));
    
    // Determine search radius in bins
    let bins_per_influence = max(1, i32(params.max_influence_radius / params.bin_cell_size.x));
    
    var total_intensity = 0.0;
    let max_influence_radius_sq = params.max_influence_radius * params.max_influence_radius;
    
    // Check neighboring bins within influence radius
    for (var dy = -bins_per_influence; dy <= bins_per_influence; dy++) {
        for (var dx = -bins_per_influence; dx <= bins_per_influence; dx++) {
            let bin_x = center_bin_x + dx;
            let bin_y = center_bin_y + dy;
            
            // Check if bin is within grid bounds
            if (bin_x < 0 || bin_x >= i32(params.grid_size.x) || 
                bin_y < 0 || bin_y >= i32(params.grid_size.y)) {
                continue;
            }
            
            let bin_idx = u32(bin_y) * params.grid_size.x + u32(bin_x);
            let bin_start = bin_offsets[bin_idx];
            let bin_count = bin_counts[bin_idx];
            
            // Process all particles in this bin
            for (var i = 0u; i < bin_count; i++) {
                let data_idx = bin_start + i;
                if (data_idx >= arrayLength(&bin_data)) {
                    continue;
                }
                
                let particle_idx = bin_data[data_idx];
                if (particle_idx >= params.particle_count) {
                    continue;
                }
                
                let particle_pos = particles[particle_idx].position;
                
                // Skip invalid particles
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
                
                // Gaussian falloff
                let sigma = 25.0;
                let influence = exp(-distance_sq / (2.0 * sigma * sigma));
                total_intensity += influence;
                
                // Early exit if we have enough intensity
                if (total_intensity > 3.0) {
                    break;
                }
            }
            
            // Early exit if we already have high intensity
            if (total_intensity > 3.0) {
                break;
            }
        }
        
        if (total_intensity > 3.0) {
            break;
        }
    }
    
    // Skip pixels with very low intensity
    if (total_intensity < 0.001) {
        textureStore(heatmap_texture, pixel_coords, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }
    
    // Normalize and apply color mapping
    let intensity = clamp(total_intensity * params.intensity_scale, 0.0, 1.0);
    var color = color_from_intensity(intensity);
    
    // Smooth alpha transition for very low intensities to reduce pop-in
    if (total_intensity < 0.05) {
        let alpha_multiplier = smoothstep(0.001, 0.05, total_intensity);
        color.a *= alpha_multiplier;
    }
    
    textureStore(heatmap_texture, pixel_coords, color);
}
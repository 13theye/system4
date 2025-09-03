// src/view/mask_manager.rs
//
// High-level mask management system that bridges System4's Mask with nnpipe's RenderWindow
// and effect preset system

use super::masks::Mask;
use crate::voice::Voice;
use nannou::prelude::*;
use nannou::wgpu;
use nnpipe::{EffectPresets, PipelineComponent, RenderWindow, TextureConfig};
use std::collections::HashMap;

/// High-level mask management system for System4
pub struct MaskManager {
    masks: HashMap<u32, ManagedMask>,
    effect_presets: EffectPresets,
    screen_size: Vec2,
    texture_config: TextureConfig,
    next_mask_id: u32,
}

/// A managed mask that combines System4's Mask with nnpipe's RenderWindow
struct ManagedMask {
    id: u32,
    mask: Mask,
    render_window: RenderWindow,
    current_effect: Option<String>,
    effect_parameters: HashMap<String, f32>,
    enabled: bool,
    layer: u32,
}

impl MaskManager {
    pub fn new(screen_size: Vec2, texture_config: TextureConfig) -> Self {
        Self {
            masks: HashMap::new(),
            effect_presets: EffectPresets::new(),
            screen_size,
            texture_config,
            next_mask_id: 1,
        }
    }

    /// Create a new mask with specified bounds and optional effect preset
    pub fn create_mask(
        &mut self,
        device: &wgpu::Device,
        voice: Voice,
        bounds: Rect,
        effect_preset: Option<&str>,
        layer: u32,
    ) -> Result<u32, String> {
        let mask_id = self.next_mask_id;
        self.next_mask_id += 1;

        // Create the basic mask
        let mut mask = Mask::full_screen(voice);
        mask.rect = bounds;
        mask.origin = bounds.xy();

        // Create the render window
        let mut render_window = RenderWindow::with_id(
            device,
            self.texture_config,
            mask_id,
            bounds,
            self.screen_size,
        );
        render_window.set_layer(layer);

        // Apply effect preset if specified
        if let Some(preset_name) = effect_preset {
            let pipeline = self
                .effect_presets
                .create_pipeline(preset_name, self.texture_config, device)
                .map_err(|e| format!("Failed to create effect pipeline: {}", e))?;

            render_window.add_effect_pipeline(preset_name.to_string(), pipeline);
            render_window.set_active_effect(Some(preset_name.to_string()));
        }

        // Create intermediate texture if needed
        if render_window.has_active_effect() {
            render_window.create_intermediate_texture(device);
        }

        let managed_mask = ManagedMask {
            id: mask_id,
            mask,
            render_window,
            current_effect: effect_preset.map(|s| s.to_string()),
            effect_parameters: HashMap::new(),
            enabled: true,
            layer,
        };

        self.masks.insert(mask_id, managed_mask);
        println!("Mask created: {}", mask_id);
        Ok(mask_id)
    }

    /// Remove a mask by ID
    pub fn remove_mask(&mut self, mask_id: u32) -> bool {
        self.masks.remove(&mask_id).is_some()
    }

    /// Get a mask by ID
    pub fn get_mask(&self, mask_id: u32) -> Option<&Mask> {
        self.masks.get(&mask_id).map(|m| &m.mask)
    }

    /// Get a mutable mask by ID
    pub fn get_mask_mut(&mut self, mask_id: u32) -> Option<&mut Mask> {
        self.masks.get_mut(&mask_id).map(|m| &mut m.mask)
    }

    /// Resize a mask to new bounds
    pub fn resize_mask(
        &mut self,
        device: &wgpu::Device,
        mask_id: u32,
        new_bounds: Rect,
    ) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        managed_mask.mask.rect = new_bounds;
        managed_mask.mask.origin = new_bounds.xy();
        managed_mask
            .render_window
            .resize_viewport(device, new_bounds, self.texture_config);

        Ok(())
    }

    /// Animate a mask to new bounds over specified duration
    pub fn animate_mask(
        &mut self,
        mask_id: u32,
        new_bounds: Rect,
        duration_secs: f32,
    ) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        managed_mask.mask.change_bounds(new_bounds, duration_secs);
        Ok(())
    }

    /// Set effect preset for a mask
    pub fn set_mask_effect(
        &mut self,
        device: &wgpu::Device,
        mask_id: u32,
        effect_preset: Option<&str>,
    ) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        if let Some(preset_name) = effect_preset {
            // Create new pipeline with current parameters if available
            let pipeline = if managed_mask.effect_parameters.is_empty() {
                self.effect_presets
                    .create_pipeline(preset_name, self.texture_config, device)?
            } else {
                self.effect_presets.create_pipeline_with_params(
                    preset_name,
                    self.texture_config,
                    &managed_mask.effect_parameters,
                    device,
                )?
            };

            managed_mask
                .render_window
                .add_effect_pipeline(preset_name.to_string(), pipeline);
            managed_mask
                .render_window
                .set_active_effect(Some(preset_name.to_string()));
            managed_mask.current_effect = Some(preset_name.to_string());

            // Ensure intermediate texture exists
            managed_mask
                .render_window
                .create_intermediate_texture(device);
        } else {
            // Remove effect
            managed_mask.render_window.set_active_effect(None);
            managed_mask.current_effect = None;
        }

        Ok(())
    }

    /// Set effect parameter for a mask
    pub fn set_effect_parameter(
        &mut self,
        device: &wgpu::Device,
        mask_id: u32,
        parameter_name: &str,
        value: f32,
    ) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        // Update parameter value
        managed_mask
            .effect_parameters
            .insert(parameter_name.to_string(), value);

        // If there's an active effect, rebuild the pipeline with new parameters
        if let Some(ref effect_name) = managed_mask.current_effect.clone() {
            let pipeline = self.effect_presets.create_pipeline_with_params(
                &effect_name,
                self.texture_config,
                &managed_mask.effect_parameters,
                device,
            )?;

            managed_mask
                .render_window
                .remove_effect_pipeline(&effect_name);
            managed_mask
                .render_window
                .add_effect_pipeline(effect_name.to_string(), pipeline);
            managed_mask
                .render_window
                .set_active_effect(Some(effect_name.to_string()));
        }

        Ok(())
    }

    /// Get current effect parameter value
    pub fn get_effect_parameter(&self, mask_id: u32, parameter_name: &str) -> Option<f32> {
        self.masks
            .get(&mask_id)
            .and_then(|m| m.effect_parameters.get(parameter_name))
            .copied()
    }

    /// Enable or disable a mask
    pub fn set_mask_enabled(&mut self, mask_id: u32, enabled: bool) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        managed_mask.enabled = enabled;
        managed_mask.render_window.set_enabled(enabled);
        Ok(())
    }

    /// Set mask layer (z-order)
    pub fn set_mask_layer(&mut self, mask_id: u32, layer: u32) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        managed_mask.layer = layer;
        managed_mask.render_window.set_layer(layer);
        Ok(())
    }

    /// Update all masks (handles animations)
    pub fn update(&mut self, device: &wgpu::Device) {
        for managed_mask in self.masks.values_mut() {
            // Update mask animation
            let animation_complete = managed_mask.mask.update_animation();

            // If animation changed the bounds, update the render window
            if !animation_complete {
                let new_bounds = managed_mask.mask.rect;
                managed_mask
                    .render_window
                    .resize_viewport(device, new_bounds, self.texture_config);
            }
        }
    }

    /// Encode all enabled masks in layer order
    pub fn encode_masks(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        // Get masks sorted by layer
        let mut mask_refs: Vec<_> = self.masks.values_mut().filter(|m| m.enabled).collect();
        mask_refs.sort_by_key(|m| m.layer);

        // Process each mask
        for managed_mask in mask_refs {
            // First encode the viewport pass
            managed_mask
                .render_window
                .finalize_bind_groups(device, input_view, None);
            managed_mask.render_window.encode_pass(encoder, output_view);

            // Then encode effects if any
            managed_mask
                .render_window
                .encode_effects(device, encoder, output_view);
        }
    }

    /// Get all mask IDs
    pub fn get_mask_ids(&self) -> Vec<u32> {
        self.masks.keys().copied().collect()
    }

    /// Get masks by voice
    pub fn get_masks_for_voice(&self, voice: Voice) -> Vec<u32> {
        self.masks
            .iter()
            .filter(|(_, m)| m.mask.voice == voice)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get available effect presets
    pub fn get_available_effects(&self) -> Vec<&String> {
        self.effect_presets.list_presets()
    }

    /// Get current effect for a mask
    pub fn get_current_effect(&self, mask_id: u32) -> Option<&String> {
        self.masks
            .get(&mask_id)
            .and_then(|m| m.current_effect.as_ref())
    }

    /// Check if mask exists
    pub fn has_mask(&self, mask_id: u32) -> bool {
        self.masks.contains_key(&mask_id)
    }

    /// Get total number of masks
    pub fn mask_count(&self) -> usize {
        self.masks.len()
    }

    /// Create a standard drone mask (convenience method)
    pub fn create_drone_mask(
        &mut self,
        device: &wgpu::Device,
        voice: Voice,
        effect_preset: Option<&str>,
    ) -> Result<u32, String> {
        let bounds = match voice {
            Voice::Voice1 => Rect::from_x_y_w_h(-1280.0, 200.0, 800.0, 1200.0),
            Voice::Voice4 => Rect::from_x_y_w_h(1280.0, 200.0, 800.0, 1200.0),
            _ => Rect::from_x_y_w_h(0.0, 0.0, 800.0, 1200.0),
        };

        self.create_mask(device, voice, bounds, effect_preset, 0)
    }

    /// Create a fullscreen mask (convenience method)
    pub fn create_fullscreen_mask(
        &mut self,
        device: &wgpu::Device,
        voice: Voice,
        effect_preset: Option<&str>,
    ) -> Result<u32, String> {
        let bounds = Rect::from_x_y_w_h(0.0, 0.0, self.screen_size.x, self.screen_size.y);
        self.create_mask(device, voice, bounds, effect_preset, 0)
    }
}

impl ManagedMask {
    /// Check if the mask contains a point
    pub fn contains(&self, point: Vec2) -> bool {
        self.mask.contains(point)
    }
}

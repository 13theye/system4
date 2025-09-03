// src/view/mask_manager.rs
//
// High-level mask management system that bridges System4's Mask with nnpipe's RenderWindow
// and effect preset system

use super::masks::Mask;
use crate::voice::Voice;
use nannou::prelude::*;
use nannou::wgpu;
use nnpipe::{Pipeline, PipelineBuilder, TextureConfig};
use std::{cell::RefCell, collections::HashMap};

/// High-level mask management system for System4
pub struct MaskManager {
    masks: HashMap<u32, ManagedMask>,
    screen_size: Vec2,
    texture_config: TextureConfig,
    next_mask_id: u32,
}

/// A managed mask that combines System4's Mask with nnpipe's Pipeline
struct ManagedMask {
    id: u32,
    mask: Mask,
    pipeline: Option<RefCell<Pipeline>>,
    current_effect: Option<String>,
    effect_parameters: HashMap<String, f32>,
    enabled: bool,
    layer: u32,
}

impl MaskManager {
    pub fn new(screen_size: Vec2, texture_config: TextureConfig) -> Self {
        Self {
            masks: HashMap::new(),
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

        // Create pipeline if effect preset specified
        let pipeline = if let Some(effect_name) = effect_preset {
            Some(RefCell::new(self.create_mask_pipeline(
                device,
                bounds,
                effect_name,
                &HashMap::new(),
            )?))
        } else {
            None
        };

        let managed_mask = ManagedMask {
            id: mask_id,
            mask,
            pipeline,
            current_effect: effect_preset.map(|s| s.to_string()),
            effect_parameters: HashMap::new(),
            enabled: true,
            layer,
        };

        self.masks.insert(mask_id, managed_mask);
        println!("Mask created: {}", mask_id);
        Ok(mask_id)
    }

    /// Create a pipeline for a mask with the given effect and parameters
    fn create_mask_pipeline(
        &self,
        device: &wgpu::Device,
        bounds: Rect,
        effect_name: &str,
        parameters: &HashMap<String, f32>,
    ) -> Result<Pipeline, String> {
        Self::create_mask_pipeline_static(
            device,
            bounds,
            effect_name,
            parameters,
            self.texture_config,
        )
    }

    /// Static version of create_mask_pipeline to avoid borrowing issues
    fn create_mask_pipeline_static(
        device: &wgpu::Device,
        _bounds: Rect,
        effect_name: &str,
        parameters: &HashMap<String, f32>,
        texture_config: TextureConfig,
    ) -> Result<Pipeline, String> {
        // For now, we'll create a simple pipeline based on the effect name
        // Later this can be expanded to use viewport/scissoring and more complex effects
        let mut builder = PipelineBuilder::new()
            .name(&format!("Mask {} Pipeline", effect_name))
            .input_texture("effects_output");

        // Apply the effect based on name and parameters
        builder = match effect_name {
            "invert" => {
                let darken_darks = parameters.get("darken_darks").unwrap_or(&1.0);
                builder.inversion(texture_config, *darken_darks)
            }
            "blur" => {
                let radius = parameters.get("radius").unwrap_or(&2.0);
                builder.gaussian_blur_passes(texture_config, 1, *radius, 5.0)
            }
            "glow" => {
                // Simple glow effect using brightness extraction and blur
                builder
                    .brightness_extract(texture_config, 0.7)
                    .gaussian_blur_passes(texture_config, 2, 3.0, 5.0)
            }
            _ => {
                return Err(format!("Unknown effect: {}", effect_name));
            }
        };

        // Build the pipeline
        builder
            .build(device)
            .map_err(|e| format!("Failed to build mask pipeline: {:?}", e))
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

        // Rebuild the pipeline with new bounds if there's an active effect
        if let Some(effect_name) = managed_mask.current_effect.clone() {
            let effect_parameters = managed_mask.effect_parameters.clone();
            let texture_config = self.texture_config;
            managed_mask.pipeline = Some(RefCell::new(Self::create_mask_pipeline_static(
                device,
                new_bounds,
                &effect_name,
                &effect_parameters,
                texture_config,
            )?));
        }

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

        if let Some(effect_name) = effect_preset {
            // Create new pipeline with current parameters
            let mask_rect = managed_mask.mask.rect;
            let effect_parameters = managed_mask.effect_parameters.clone();
            let texture_config = self.texture_config;
            managed_mask.pipeline = Some(RefCell::new(Self::create_mask_pipeline_static(
                device,
                mask_rect,
                effect_name,
                &effect_parameters,
                texture_config,
            )?));
            managed_mask.current_effect = Some(effect_name.to_string());
        } else {
            // Remove effect
            managed_mask.pipeline = None;
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
        if let Some(effect_name) = managed_mask.current_effect.clone() {
            let mask_rect = managed_mask.mask.rect;
            let effect_parameters = managed_mask.effect_parameters.clone();
            let texture_config = self.texture_config;
            managed_mask.pipeline = Some(RefCell::new(Self::create_mask_pipeline_static(
                device,
                mask_rect,
                &effect_name,
                &effect_parameters,
                texture_config,
            )?));
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
        Ok(())
    }

    /// Set mask layer (z-order)
    pub fn set_mask_layer(&mut self, mask_id: u32, layer: u32) -> Result<(), String> {
        let managed_mask = self
            .masks
            .get_mut(&mask_id)
            .ok_or_else(|| format!("Mask {} not found", mask_id))?;

        managed_mask.layer = layer;
        Ok(())
    }

    /// Update all masks (handles animations)
    pub fn update(&mut self, device: &wgpu::Device) {
        // Collect mask IDs that need pipeline updates
        let mut masks_to_update = Vec::new();

        for (mask_id, managed_mask) in &mut self.masks {
            // Update mask animation
            let animation_complete = managed_mask.mask.update_animation();

            // If animation changed the bounds, mark for pipeline rebuild
            if !animation_complete && managed_mask.current_effect.is_some() {
                masks_to_update.push(*mask_id);
            }
        }

        // Rebuild pipelines for masks that need updates
        for mask_id in masks_to_update {
            if let Some(managed_mask) = self.masks.get_mut(&mask_id) {
                if let Some(effect_name) = &managed_mask.current_effect.clone() {
                    let new_bounds = managed_mask.mask.rect;
                    let effect_parameters = managed_mask.effect_parameters.clone();
                    if let Ok(pipeline) = Self::create_mask_pipeline_static(
                        device,
                        new_bounds,
                        effect_name,
                        &effect_parameters,
                        self.texture_config,
                    ) {
                        managed_mask.pipeline = Some(RefCell::new(pipeline));
                    }
                }
            }
        }
    }

    /// Render masks as a pipeline step - composites all enabled masks over the input texture
    /// This method should be called during the mask_compositor pipeline execution  
    /// If no active masks exist, this method does nothing and lets the pipeline passthrough
    pub fn render_pipeline_step(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        // If no active masks, do nothing - let the pipeline handle passthrough
        if self.masks.is_empty()
            || !self
                .masks
                .values()
                .any(|m| m.enabled && m.pipeline.is_some())
        {
            return;
        }

        // Execute all enabled mask pipelines in layer order
        // For now, the last mask wins (overwrites previous ones)
        // TODO: Implement proper layer compositing with intermediate textures
        let mut mask_refs: Vec<_> = self
            .masks
            .values()
            .filter(|m| m.enabled && m.pipeline.is_some())
            .collect();
        mask_refs.sort_by_key(|m| m.layer);

        for managed_mask in mask_refs {
            if let Some(ref pipeline_cell) = managed_mask.pipeline {
                println!(
                    "Executing mask pipeline for mask {} with effect {:?}",
                    managed_mask.id, managed_mask.current_effect
                );

                // Execute the mask pipeline from input_view to output_view
                pipeline_cell
                    .borrow_mut()
                    .encode_into(device, encoder, input_view, output_view);
            }
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
    pub fn get_available_effects(&self) -> Vec<&str> {
        vec!["invert", "blur", "glow"]
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

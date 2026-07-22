//! Image zoom and pan calculations for the viewer

/// Handles zoom levels, pan offsets, and viewport dimension calculations for image viewing.
#[derive(Debug, Clone)]
pub struct ZoomCalculator {
    /// Current zoom level (1.0 to 15.0)
    pub current_zoom: f32,
    /// Viewport width in pixels
    pub viewport_width: u32,
    /// Viewport height in pixels
    pub viewport_height: u32,
    /// Original image width in pixels
    pub image_width: u32,
    /// Original image height in pixels
    pub image_height: u32,
    /// Horizontal pan offset in pixels
    pub pan_x: i32,
    /// Vertical pan offset in pixels
    pub pan_y: i32,
}

impl ZoomCalculator {
    /// Creates a new ZoomCalculator with the given image and viewport dimensions.
    ///
    /// # Arguments
    ///
    /// * `image_width` - Original image width in pixels
    /// * `image_height` - Original image height in pixels
    /// * `viewport_width` - Viewport width in pixels
    /// * `viewport_height` - Viewport height in pixels
    pub fn new(image_width: u32, image_height: u32, viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            current_zoom: 1.0,
            viewport_width,
            viewport_height,
            image_width,
            image_height,
            pan_x: 0,
            pan_y: 0,
        }
    }

    /// Increases zoom level by 0.1, clamped to maximum 15.0.
    pub fn zoom_in(&mut self) {
        self.current_zoom = (self.current_zoom + 0.1).min(15.0);
    }

    /// Decreases zoom level by 0.1, clamped to minimum 1.0.
    pub fn zoom_out(&mut self) {
        self.current_zoom = (self.current_zoom - 0.1).max(1.0);
    }

    /// Resets zoom to 1.0 and pan offsets to 0.
    pub fn reset_zoom(&mut self) {
        self.current_zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
    }

    /// Auto-scales image to fit within the viewport.
    /// Sets zoom to the appropriate level to fit the entire image.
    pub fn fit_to_window(&mut self) {
        let viewport_width_f = self.viewport_width as f32;
        let viewport_height_f = self.viewport_height as f32;
        let image_width_f = self.image_width as f32;
        let image_height_f = self.image_height as f32;

        let zoom_x = viewport_width_f / image_width_f;
        let zoom_y = viewport_height_f / image_height_f;

        let fit_zoom = zoom_x.min(zoom_y).max(0.1).min(15.0);
        self.current_zoom = fit_zoom;
        self.pan_x = 0;
        self.pan_y = 0;
    }

    /// Pans the image by the given delta values, with boundary checking.
    ///
    /// # Arguments
    ///
    /// * `dx` - Horizontal pan delta
    /// * `dy` - Vertical pan delta
    pub fn pan(&mut self, dx: i32, dy: i32) {
        let display_width = self.get_display_dimensions().0;
        let display_height = self.get_display_dimensions().1;

        let max_pan_x = ((display_width as i32) - (self.viewport_width as i32)).max(0);
        let max_pan_y = ((display_height as i32) - (self.viewport_height as i32)).max(0);

        self.pan_x = (self.pan_x + dx).max(0).min(max_pan_x);
        self.pan_y = (self.pan_y + dy).max(0).min(max_pan_y);
    }

    /// Zooms based on mouse wheel delta.
    ///
    /// # Arguments
    ///
    /// * `delta` - Mouse wheel delta (positive for zoom in, negative for zoom out)
    #[allow(dead_code)]
    pub fn mouse_wheel_zoom(&mut self, delta: i32) {
        if delta > 0 {
            self.zoom_in();
        } else if delta < 0 {
            self.zoom_out();
        }
    }

    /// Calculates the displayed image dimensions based on the current zoom level.
    ///
    /// # Returns
    ///
    /// A tuple `(width, height)` of the displayed image size in pixels
    pub fn get_display_dimensions(&self) -> (u32, u32) {
        let display_width = (self.image_width as f32 * self.current_zoom) as u32;
        let display_height = (self.image_height as f32 * self.current_zoom) as u32;
        (display_width, display_height)
    }

    /// Returns the current zoom level.
    #[allow(dead_code)]
    pub fn get_current_zoom(&self) -> f32 {
        self.current_zoom
    }

    /// Returns the current pan offset.
    #[allow(dead_code)]
    pub fn get_pan_offset(&self) -> (i32, i32) {
        (self.pan_x, self.pan_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_bounds() {
        let mut calculator = ZoomCalculator::new(100, 100, 200, 200);

        // Test zoom in with bounds
        for _ in 0..200 {
            calculator.zoom_in();
        }
        assert_eq!(calculator.get_current_zoom(), 15.0, "Zoom should be clamped to 15.0");

        // Test zoom out with bounds
        for _ in 0..200 {
            calculator.zoom_out();
        }
        assert_eq!(calculator.get_current_zoom(), 1.0, "Zoom should be clamped to 1.0");
    }

    #[test]
    fn test_fit_to_window() {
        let mut calculator = ZoomCalculator::new(1000, 1000, 500, 500);

        // Set zoom to something other than fit
        calculator.zoom_in();
        assert_ne!(calculator.get_current_zoom(), 0.5, "Zoom should be changed");

        // Fit to window should scale down
        calculator.fit_to_window();
        assert_eq!(calculator.get_current_zoom(), 0.5, "Zoom should be 0.5 to fit 1000x1000 image in 500x500 viewport");

        // Pan offsets should be reset
        let (pan_x, pan_y) = calculator.get_pan_offset();
        assert_eq!(pan_x, 0, "Pan X should be reset");
        assert_eq!(pan_y, 0, "Pan Y should be reset");
    }

    #[test]
    fn test_pan_bounds() {
        let mut calculator = ZoomCalculator::new(100, 100, 200, 200);

        // At 1.0 zoom with 100x100 image in 200x200 viewport, pan should be 0
        calculator.pan(100, 100);
        let (pan_x, pan_y) = calculator.get_pan_offset();
        assert_eq!(pan_x, 0, "Pan X should be 0 when image fits in viewport");
        assert_eq!(pan_y, 0, "Pan Y should be 0 when image fits in viewport");

        // Zoom in to create panning space
        calculator.current_zoom = 2.0; // 200x200 displayed size in 200x200 viewport

        // Pan should be limited to image boundaries
        calculator.pan(1000, 1000);
        let (pan_x, pan_y) = calculator.get_pan_offset();
        assert!(pan_x <= 200, "Pan X should not exceed display width");
        assert!(pan_y <= 200, "Pan Y should not exceed display height");

        // Reset and try panning in negative direction
        calculator.pan_x = 0;
        calculator.pan_y = 0;
        calculator.pan(-100, -100);
        let (pan_x, pan_y) = calculator.get_pan_offset();
        assert_eq!(pan_x, 0, "Pan X should be clamped to 0 minimum");
        assert_eq!(pan_y, 0, "Pan Y should be clamped to 0 minimum");
    }
}

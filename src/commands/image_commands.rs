use crate::image_zoom::ZoomCalculator;
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn zoom_in(
    file_id: String,
    zoom_state: State<'_, Mutex<HashMap<String, ZoomCalculator>>>,
) -> Result<f32, String> {
    let mut state = zoom_state.lock().map_err(|e| e.to_string())?;
    let calc = state.entry(file_id).or_insert_with(|| {
        ZoomCalculator::new(800, 600, 400, 300)
    });
    calc.zoom_in();
    Ok(calc.current_zoom)
}

#[tauri::command]
pub async fn zoom_out(
    file_id: String,
    zoom_state: State<'_, Mutex<HashMap<String, ZoomCalculator>>>,
) -> Result<f32, String> {
    let mut state = zoom_state.lock().map_err(|e| e.to_string())?;
    let calc = state.entry(file_id).or_insert_with(|| {
        ZoomCalculator::new(800, 600, 400, 300)
    });
    calc.zoom_out();
    Ok(calc.current_zoom)
}

#[tauri::command]
pub async fn reset_zoom(
    file_id: String,
    zoom_state: State<'_, Mutex<HashMap<String, ZoomCalculator>>>,
) -> Result<(), String> {
    let mut state = zoom_state.lock().map_err(|e| e.to_string())?;
    if let Some(calc) = state.get_mut(&file_id) {
        calc.reset_zoom();
    }
    Ok(())
}

#[tauri::command]
pub async fn fit_to_window(
    file_id: String,
    zoom_state: State<'_, Mutex<HashMap<String, ZoomCalculator>>>,
) -> Result<f32, String> {
    let mut state = zoom_state.lock().map_err(|e| e.to_string())?;
    let calc = state.entry(file_id).or_insert_with(|| {
        ZoomCalculator::new(800, 600, 400, 300)
    });
    calc.fit_to_window();
    Ok(calc.current_zoom)
}

#[tauri::command]
pub async fn pan(
    file_id: String,
    dx: i32,
    dy: i32,
    zoom_state: State<'_, Mutex<HashMap<String, ZoomCalculator>>>,
) -> Result<(i32, i32), String> {
    let mut state = zoom_state.lock().map_err(|e| e.to_string())?;
    let calc = state.entry(file_id).or_insert_with(|| {
        ZoomCalculator::new(800, 600, 400, 300)
    });
    calc.pan(dx, dy);
    Ok((calc.pan_x, calc.pan_y))
}

// agent/src/input_tracking.rs
// Keyboard and Mouse Activity Tracking with Heatmap Generation
// Monitors user input activity and generates activity heatmaps

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};
use serde_json::json;

/// Represents a single grid cell in the heatmap
#[derive(Debug, Clone, Default)]
pub struct GridCell {
    pub x: u32,
    pub y: u32,
    pub activity_count: u32,
}

/// Input activity statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct InputStats {
    pub mouse_moves: u32,
    pub mouse_clicks: u32,
    pub keyboard_events: u32,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Heatmap generated from aggregated input events
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivityHeatmap {
    pub timestamp: DateTime<Utc>,
    pub device_id: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub grid_data: HashMap<String, u32>,  // Format: "x,y" -> count
    pub stats: InputStats,
}

/// Main input tracker for monitoring keyboard and mouse activity
pub struct InputTracker {
    heatmap: Arc<Mutex<ActivityHeatmap>>,
    last_upload: Arc<Mutex<DateTime<Utc>>>,
    grid_resolution: u32,
    upload_interval: Duration,
}

impl InputTracker {
    pub fn new(device_id: String, grid_resolution: u32) -> Self {
        let now = Utc::now();
        
        Self {
            heatmap: Arc::new(Mutex::new(ActivityHeatmap {
                timestamp: now,
                device_id,
                screen_width: 1920,  // Default, will be updated
                screen_height: 1080, // Default, will be updated
                grid_data: HashMap::new(),
                stats: InputStats::default(),
            })),
            last_upload: Arc::new(Mutex::new(now)),
            grid_resolution,
            upload_interval: Duration::hours(1),
        }
    }

    /// Record mouse movement at coordinates
    pub async fn record_mouse_movement(&self, x: u32, y: u32) {
        if let Ok(mut heatmap) = self.heatmap.lock().await {
            let grid_x = (x / self.grid_resolution).min(99);  // Max 100 cells
            let grid_y = (y / self.grid_resolution).min(99);
            let key = format!("{},{}", grid_x, grid_y);
            
            heatmap.grid_data
                .entry(key)
                .and_modify(|count| *count += 1)
                .or_insert(1);
            
            heatmap.stats.mouse_moves += 1;
            heatmap.stats.last_activity = Some(Utc::now());
        }
    }

    /// Record mouse click
    pub async fn record_mouse_click(&self, x: u32, y: u32) {
        self.record_mouse_movement(x, y).await;
        
        if let Ok(mut heatmap) = self.heatmap.lock().await {
            heatmap.stats.mouse_clicks += 1;
        }
    }

    /// Record keyboard event
    pub async fn record_keyboard_event(&self, _key: &str) {
        if let Ok(mut heatmap) = self.heatmap.lock().await {
            heatmap.stats.keyboard_events += 1;
            heatmap.stats.last_activity = Some(Utc::now());
        }
    }

    /// Update screen resolution
    pub async fn set_screen_resolution(&self, width: u32, height: u32) {
        if let Ok(mut heatmap) = self.heatmap.lock().await {
            heatmap.screen_width = width;
            heatmap.screen_height = height;
        }
    }

    /// Check if it's time to upload the heatmap
    pub async fn should_upload(&self) -> bool {
        if let Ok(last_upload) = self.last_upload.lock().await {
            let elapsed = Utc::now() - *last_upload;
            elapsed >= self.upload_interval
        } else {
            false
        }
    }

    /// Get current heatmap for upload
    pub async fn get_heatmap_for_upload(&self) -> Option<ActivityHeatmap> {
        if let Ok(mut heatmap) = self.heatmap.lock().await {
            // Only upload if there's activity
            if heatmap.stats.mouse_moves > 0 || heatmap.stats.keyboard_events > 0 {
                let heatmap_to_upload = heatmap.clone();
                
                // Reset for next period
                heatmap.timestamp = Utc::now();
                heatmap.grid_data.clear();
                heatmap.stats = InputStats::default();
                
                // Update last upload time
                if let Ok(mut last_upload) = self.last_upload.lock().await {
                    *last_upload = Utc::now();
                }
                
                return Some(heatmap_to_upload);
            }
        }
        None
    }

    /// Get current statistics without uploading
    pub async fn get_stats(&self) -> InputStats {
        if let Ok(heatmap) = self.heatmap.lock().await {
            heatmap.stats.clone()
        } else {
            InputStats::default()
        }
    }

    /// Get heatmap for dashboard display (current period so far)
    pub async fn get_current_heatmap_data(&self) -> serde_json::Value {
        if let Ok(heatmap) = self.heatmap.lock().await {
            json!({
                "timestamp": heatmap.timestamp.to_rfc3339(),
                "device_id": heatmap.device_id,
                "screen_width": heatmap.screen_width,
                "screen_height": heatmap.screen_height,
                "grid_data": heatmap.grid_data,
                "stats": {
                    "mouse_moves": heatmap.stats.mouse_moves,
                    "mouse_clicks": heatmap.stats.mouse_clicks,
                    "keyboard_events": heatmap.stats.keyboard_events,
                },
                "heatmap_generated_at": Utc::now().to_rfc3339(),
            })
        } else {
            json!(null)
        }
    }
}

/// Optional: Simulated input event (for testing without input devices)
#[cfg(test)]
pub async fn simulate_user_activity(tracker: &InputTracker) {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    
    // Simulate 100 mouse movements
    for _ in 0..100 {
        let x = rng.gen_range(0..1920);
        let y = rng.gen_range(0..1080);
        tracker.record_mouse_movement(x, y).await;
    }
    
    // Simulate 20 clicks
    for _ in 0..20 {
        let x = rng.gen_range(0..1920);
        let y = rng.gen_range(0..1080);
        tracker.record_mouse_click(x, y).await;
    }
    
    // Simulate 50 keyboard events
    for _ in 0..50 {
        tracker.record_keyboard_event("key").await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_input_tracking() {
        let tracker = InputTracker::new("test-device".to_string(), 19);
        
        // Simulate activity
        tracker.record_mouse_movement(500, 600).await;
        tracker.record_mouse_click(1000, 800).await;
        tracker.record_keyboard_event("a").await;
        
        // Check stats
        let stats = tracker.get_stats().await;
        assert_eq!(stats.mouse_moves, 2);  // Movement + Click
        assert_eq!(stats.mouse_clicks, 1);
        assert_eq!(stats.keyboard_events, 1);
    }

    #[tokio::test]
    async fn test_heatmap_generation() {
        let tracker = InputTracker::new("test-device".to_string(), 19);
        tracker.set_screen_resolution(1920, 1080).await;
        
        // Add activity
        for i in 0..50 {
            tracker.record_mouse_movement(100 + (i * 10), 200 + (i * 5)).await;
        }
        
        // Get heatmap
        let heatmap_data = tracker.get_current_heatmap_data().await;
        assert!(heatmap_data["grid_data"].is_object());
        assert!(heatmap_data["grid_data"].as_object().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_upload_interval() {
        let tracker = InputTracker::new("test-device".to_string(), 19);
        
        assert!(!tracker.should_upload().await);
        
        // Simulate time passing (in real code would be actual time)
        // For now, we just verify the mechanism exists
    }
}

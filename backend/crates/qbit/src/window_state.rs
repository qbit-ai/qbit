use qbit_settings::WindowSettings;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedWindowState {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestoreAction {
    Maximize,
    Bounds {
        width: f64,
        height: f64,
        x: Option<f64>,
        y: Option<f64>,
    },
}

pub fn normalize_persisted_window_state(
    width: f64,
    height: f64,
    x: Option<f64>,
    y: Option<f64>,
    maximized: bool,
) -> NormalizedWindowState {
    let width = width.round().max(1.0) as u32;
    let height = height.round().max(1.0) as u32;

    NormalizedWindowState {
        width,
        height,
        x: x.map(|x| x.round() as i32),
        y: y.map(|y| y.round() as i32),
        maximized,
    }
}

pub fn compute_restore_action(
    ws: &WindowSettings,
    monitor: Option<MonitorRect>,
) -> Option<RestoreAction> {
    if ws.width == 0 || ws.height == 0 {
        return None;
    }

    if ws.maximized {
        return Some(RestoreAction::Maximize);
    }

    match monitor {
        Some(monitor) => {
            let mut width = ws.width as f64;
            let mut height = ws.height as f64;

            width = width.min(monitor.width).max(1.0);
            height = height.min(monitor.height).max(1.0);

            let (x, y) = match (ws.x, ws.y) {
                (Some(x), Some(y)) => {
                    let x = (x as f64)
                        .max(monitor.x)
                        .min(monitor.x + monitor.width - width);
                    let y = (y as f64)
                        .max(monitor.y)
                        .min(monitor.y + monitor.height - height);
                    (Some(x), Some(y))
                }
                _ => (None, None),
            };

            Some(RestoreAction::Bounds {
                width,
                height,
                x,
                y,
            })
        }
        None => Some(RestoreAction::Bounds {
            width: ws.width as f64,
            height: ws.height as f64,
            x: ws.x.map(|x| x as f64),
            y: ws.y.map(|y| y as f64),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_persisted_window_state_rounds_and_clamps_size() {
        let s = normalize_persisted_window_state(800.6, 0.4, Some(10.2), Some(-20.8), false);
        assert_eq!(s.width, 801);
        assert_eq!(s.height, 1);
        assert_eq!(s.x, Some(10));
        assert_eq!(s.y, Some(-21));
        assert!(!s.maximized);
    }

    #[test]
    fn compute_restore_action_returns_none_for_zero_sizes() {
        let ws = WindowSettings {
            width: 0,
            height: 10,
            x: Some(1),
            y: Some(2),
            maximized: false,
        };
        assert_eq!(compute_restore_action(&ws, None), None);

        let ws = WindowSettings {
            width: 10,
            height: 0,
            x: Some(1),
            y: Some(2),
            maximized: false,
        };
        assert_eq!(compute_restore_action(&ws, None), None);
    }

    #[test]
    fn compute_restore_action_maximize_takes_precedence() {
        let ws = WindowSettings {
            width: 800,
            height: 600,
            x: Some(50),
            y: Some(60),
            maximized: true,
        };
        assert_eq!(compute_restore_action(&ws, None), Some(RestoreAction::Maximize));

        let monitor = MonitorRect {
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 400.0,
        };
        assert_eq!(
            compute_restore_action(&ws, Some(monitor)),
            Some(RestoreAction::Maximize)
        );
    }

    #[test]
    fn compute_restore_action_clamps_to_monitor_and_keeps_window_on_screen() {
        let ws = WindowSettings {
            width: 2000,
            height: 1500,
            x: Some(900),
            y: Some(700),
            maximized: false,
        };

        let monitor = MonitorRect {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 800.0,
        };

        let action = compute_restore_action(&ws, Some(monitor));
        assert_eq!(
            action,
            Some(RestoreAction::Bounds {
                width: 1000.0,
                height: 800.0,
                x: Some(0.0),
                y: Some(0.0),
            })
        );
    }

    #[test]
    fn compute_restore_action_supports_negative_monitor_origins() {
        let ws = WindowSettings {
            width: 500,
            height: 400,
            x: Some(-5000),
            y: Some(9999),
            maximized: false,
        };

        let monitor = MonitorRect {
            x: -1440.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        };

        // x clamps to left edge (-1440). y clamps to max allowed (900 - 400 = 500).
        assert_eq!(
            compute_restore_action(&ws, Some(monitor)),
            Some(RestoreAction::Bounds {
                width: 500.0,
                height: 400.0,
                x: Some(-1440.0),
                y: Some(500.0),
            })
        );
    }

    #[test]
    fn compute_restore_action_without_position_leaves_it_unset() {
        let ws = WindowSettings {
            width: 800,
            height: 600,
            x: None,
            y: Some(10),
            maximized: false,
        };

        let monitor = MonitorRect {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 800.0,
        };

        assert_eq!(
            compute_restore_action(&ws, Some(monitor)),
            Some(RestoreAction::Bounds {
                width: 800.0,
                height: 600.0,
                x: None,
                y: None,
            })
        );
    }
}

// Tarjeta animada que aparece al conectar los AirPods (estilo iPhone).
use crate::bluetooth::aacp::{BatteryComponent, BatteryInfo, BatteryStatus};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, path::Arc, Frame, Geometry, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{mouse, Color, Pixels, Point, Radians, Rectangle, Renderer, Theme};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

/// Cuánto dura la animación de entrada y cuánto se queda abierta.
pub const ANIM: Duration = Duration::from_millis(1100);
pub const VISIBLE: Duration = Duration::from_millis(6500);
pub const SIZE: (f32, f32) = (400.0, 230.0);

pub struct Popup {
    pub name: String,
    pub battery: Vec<BatteryInfo>,
    pub started: Instant,
    pub now: Instant,
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

impl Popup {
    pub fn elapsed(&self) -> f32 {
        self.now.duration_since(self.started).as_secs_f32()
    }

    pub fn finished(&self) -> bool {
        self.now.duration_since(self.started) >= VISIBLE
    }

    fn items(&self) -> Vec<(&'static str, Option<u8>, bool)> {
        let get = |c: BatteryComponent| {
            self.battery
                .iter()
                .find(|b| b.component == c && b.status != BatteryStatus::Disconnected)
                .map(|b| (b.level, b.status == BatteryStatus::Charging))
        };
        if let Some((l, ch)) = get(BatteryComponent::Headphone) {
            return vec![("Audífonos", Some(l), ch)];
        }
        let mut v = Vec::new();
        for (label, c) in [
            ("Izquierdo", BatteryComponent::Left),
            ("Derecho", BatteryComponent::Right),
            ("Estuche", BatteryComponent::Case),
        ] {
            match get(c) {
                Some((l, ch)) => v.push((label, Some(l), ch)),
                None => v.push((label, None, false)),
            }
        }
        v
    }
}

impl<Message> canvas::Program<Message> for Popup {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let t = self.elapsed();

        // Entrada: la tarjeta sube y aparece; salida: se desvanece el último medio segundo.
        let enter = ease_out_back(t / 0.45);
        let fade_in = ease_out_cubic(t / 0.3);
        let left = VISIBLE.as_secs_f32() - t;
        let fade_out = (left / 0.5).clamp(0.0, 1.0);
        let alpha = fade_in * fade_out;
        let dy = (1.0 - enter) * 40.0 + (1.0 - fade_out) * 15.0;

        let a = |c: Color| Color { a: c.a * alpha, ..c };

        // Fondo redondeado
        let card = Path::rounded_rectangle(
            Point::new(8.0, 8.0 + dy),
            iced::Size::new(w - 16.0, h - 16.0),
            22.0.into(),
        );
        frame.fill(&card, a(Color::from_rgba8(28, 28, 32, 0.97)));
        frame.stroke(
            &card,
            Stroke::default()
                .with_width(1.0)
                .with_color(a(Color::from_rgba8(255, 255, 255, 0.08))),
        );

        // Título
        frame.fill_text(Text {
            content: self.name.clone(),
            position: Point::new(w / 2.0, 34.0 + dy),
            color: a(Color::WHITE),
            size: Pixels(19.0),
            align_x: Alignment::Center,
            align_y: Vertical::Center,
            ..Text::default()
        });
        frame.fill_text(Text {
            content: "Conectados".into(),
            position: Point::new(w / 2.0, 56.0 + dy),
            color: a(Color::from_rgb8(140, 140, 150)),
            size: Pixels(13.0),
            align_x: Alignment::Center,
            align_y: Vertical::Center,
            ..Text::default()
        });

        // Anillos de batería, uno tras otro con pequeño retraso
        let items = self.items();
        let n = items.len() as f32;
        let radius = 34.0;
        for (i, (label, level, charging)) in items.iter().enumerate() {
            let cx = w * (i as f32 + 0.5) / n;
            let cy = 125.0 + dy;
            let p = ease_out_cubic((t - 0.15 - i as f32 * 0.12) / (ANIM.as_secs_f32() - 0.3));
            let pop = ease_out_back((t - 0.1 - i as f32 * 0.12) / 0.4).max(0.0);
            let r = radius * (0.6 + 0.4 * pop);

            let track = Path::circle(Point::new(cx, cy), r);
            frame.stroke(
                &track,
                Stroke::default()
                    .with_width(7.0)
                    .with_color(a(Color::from_rgb8(58, 58, 64))),
            );

            let (pct_text, color) = match level {
                Some(l) => {
                    let shown = (*l as f32 * p).round() as u8;
                    let color = if *charging || *l > 20 {
                        Color::from_rgb8(52, 199, 89)
                    } else if *l > 10 {
                        Color::from_rgb8(255, 159, 10)
                    } else {
                        Color::from_rgb8(255, 69, 58)
                    };
                    if *l > 0 && p > 0.0 {
                        let start = -PI / 2.0;
                        let end = start + 2.0 * PI * (*l as f32 / 100.0) * p;
                        let arc = Path::new(|b| {
                            b.arc(Arc {
                                center: Point::new(cx, cy),
                                radius: r,
                                start_angle: Radians(start),
                                end_angle: Radians(end),
                            })
                        });
                        frame.stroke(
                            &arc,
                            Stroke::default()
                                .with_width(7.0)
                                .with_line_cap(canvas::LineCap::Round)
                                .with_color(a(color)),
                        );
                    }
                    (format!("{}%", shown), color)
                }
                None => ("—".to_string(), Color::from_rgb8(110, 110, 120)),
            };

            frame.fill_text(Text {
                content: pct_text,
                position: Point::new(cx, cy),
                color: a(if level.is_some() { Color::WHITE } else { color }),
                size: Pixels(17.0),
                align_x: Alignment::Center,
                align_y: Vertical::Center,
                ..Text::default()
            });
            frame.fill_text(Text {
                content: format!("{}{}", label, if *charging { " ⚡" } else { "" }),
                position: Point::new(cx, cy + radius + 20.0),
                color: a(Color::from_rgb8(170, 170, 180)),
                size: Pixels(13.0),
                align_x: Alignment::Center,
                align_y: Vertical::Center,
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

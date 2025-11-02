use eframe::egui;
use rand::Rng;
use std::f32::consts::PI;
use std::time::Duration;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Triangle Size Slider",
        options,
        Box::new(|_cc| {
            Ok(Box::new(TriangleDrawer {
                size: 100.0,
                angle: 0.0,
                last_change: 0,
                frequency: 5.0,
                tilt_direction: egui::vec2(1.0, 0.0),
            }))
        }),
    )
}

struct TriangleDrawer {
    size: f32,
    angle: f32,
    last_change: i32,
    frequency: f32,
    tilt_direction: egui::Vec2,
}

impl eframe::App for TriangleDrawer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = ctx.input(|i| i.time) as f32;
        let mut rng = rand::rng();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Triangle with Adjustable Side");

            // Slider to control side length
            ui.add(egui::Slider::new(&mut self.size, 20.0..=200.0).text("Size"));
            ui.add(egui::Slider::new(&mut self.frequency, 1.0..=20.0).text("Frequency"));
            //ui.add(egui::Slider::new(&mut self.angle, 0.0..=1.0).text("Turns"));
            ui.label(format!("Time: {:.1} s", t));

            let frequency = self.frequency;

            self.angle = t / (frequency * 5.0) ;

            if t - self.last_change as f32 > frequency {
                self.last_change = t.trunc() as i32;
                self.tilt_direction = egui::Vec2::new(rng.random(), rng.random()); // each in [0,
            }

            // Reserve drawing space
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(300.0, 300.0), egui::Sense::hover());

            let painter = ui.painter_at(rect);

            let triangle = make_triangle(self.size, self.angle * 2.0 * PI);

            let magnitude = ((t - self.last_change as f32 - 0.5 * frequency).abs() - 0.5 * frequency) * self.size / 100.0;

            let tilt = self.size * magnitude * self.tilt_direction / 5.0;

            let center = rect.center() + tilt;

            painter.add(triangle_to_polygon(center, triangle));
        });

        ctx.request_repaint_after(Duration::from_millis(16)); // ~60 FPS
    }
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    v1: egui::Vec2,
    v2: egui::Vec2,
    v3: egui::Vec2,
}

fn triangle_to_polygon(center: egui::Pos2, tri: Triangle) -> egui::Shape {
    egui::Shape::convex_polygon(
        vec![center + tri.v1, center + tri.v2, center + tri.v3],
        egui::Color32::from_rgb(200, 100, 100),
        egui::Stroke::new(2.0, egui::Color32::BLACK),
    )
}

fn make_triangle(size: f32, angle: f32) -> Triangle {
    Triangle {
        v1: size * egui::Vec2::angled(angle + 2.0 * PI * (1.0 / 4.0)),
        v2: size * egui::Vec2::angled(angle + 2.0 * PI * (1.0 / 4.0 + 1.0 / 3.0)),
        v3: size * egui::Vec2::angled(angle + 2.0 * PI * (1.0 / 4.0 + 2.0 / 3.0)),
    }
}

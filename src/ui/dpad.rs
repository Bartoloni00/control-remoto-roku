use crate::roku::{Cmd, Dir};
use eframe::egui::{
    self, epaint::{Mesh, Vertex, WHITE_UV}, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2
};
use std::f32::consts::TAU;

pub fn render(
    ui: &mut egui::Ui,
    mouse_down: &mut [bool; 4],
    button_color: Color32,
    button_hover: Color32,
    button_down: Color32,
    orange: (u8, u8, u8),
    violet: (u8, u8, u8),
    blue: (u8, u8, u8),
    send_cmd: impl Fn(Cmd),
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(340.0), Sense::hover());
    let c = rect.center();
    let painter = ui.painter().clone();

    let trefoil_bounds = Rect::from_center_size(c, Vec2::splat(300.0));
    trefoil(&painter, c, 150.0, trefoil_bounds, orange, violet, blue);

    let btn = 84.0;
    let off = 96.0;
    let centers = [
        c + Vec2::new(0.0, -off),
        c + Vec2::new(0.0, off),
        c + Vec2::new(-off, 0.0),
        c + Vec2::new(off, 0.0),
    ];
    let dirs = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)];

    for (i, d) in Dir::ALL.iter().enumerate() {
        let r = Rect::from_center_size(centers[i], Vec2::splat(btn));
        let resp = ui.interact(r, ui.id().with(("dir", i)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let pressed = resp.is_pointer_button_down_on();
        mouse_down[i] = pressed;

        let fill = if pressed {
            button_down
        } else if resp.hovered() {
            button_hover
        } else {
            button_color
        };
        painter.rect_filled(r, 20.0, fill);
        let (dx, dy) = dirs[i];
        let _ = d;
        triangle(&painter, r.center(), Vec2::new(dx, dy), 16.0);
    }

    let ok_bounds = Rect::from_center_size(c, Vec2::splat(80.0));
    let ok_resp = ui.interact(ok_bounds, ui.id().with("ok"), Sense::click());

    if ok_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    gradient_circle(&painter, c, 40.0, ok_bounds, orange, violet, blue);
    painter.text(
        c,
        Align2::CENTER_CENTER,
        "OK",
        FontId::proportional(20.0),
        Color32::WHITE,
    );
    if ok_resp.clicked() {
        send_cmd(Cmd::Press("Select"));
    }
}

// Funciones auxiliares de pintura del D-pad
fn triangle(p: &Painter, c: Pos2, d: Vec2, s: f32) {
    let n = Vec2::new(-d.y, d.x);
    let tip = c + d * s;
    let base = c - d * s * 0.5;
    let pts = vec![tip, base + n * s * 0.8, base - n * s * 0.8];
    p.add(Shape::convex_polygon(pts, Color32::WHITE, Stroke::NONE));
}

fn trefoil(p: &Painter, c: Pos2, r: f32, bounds: Rect, orange: (u8, u8, u8), violet: (u8, u8, u8), blue: (u8, u8, u8)) {
    const N: usize = 180;
    let pts: Vec<Pos2> = (0..=N)
        .map(|i| {
            let t = i as f32 / N as f32 * TAU;
            let rad = r * (0.8 + 0.2 * (3.0 * t).cos());
            c + Vec2::new(rad * t.cos(), rad * t.sin())
        })
        .collect();
    for w in pts.windows(2) {
        let mid = Pos2::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
        p.line_segment([w[0], w[1]], Stroke::new(4.0_f32, gradient_at(mid, bounds, orange, violet, blue)));
    }
}

fn gradient_circle(p: &Painter, c: Pos2, r: f32, bounds: Rect, orange: (u8, u8, u8), violet: (u8, u8, u8), blue: (u8, u8, u8)) {
    const M: usize = 64;
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: c,
        uv: WHITE_UV,
        color: gradient_at(c, bounds, orange, violet, blue),
    });
    for i in 0..M {
        let t = i as f32 / M as f32 * TAU;
        let pos = c + Vec2::new(r * t.cos(), r * t.sin());
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color: gradient_at(pos, bounds, orange, violet, blue),
        });
    }
    for i in 0..M as u32 {
        let a = 1 + i;
        let b = 1 + (i + 1) % M as u32;
        mesh.indices.extend([0, a, b]);
    }
    p.add(Shape::mesh(mesh));
}

fn gradient_at(pos: Pos2, b: Rect, orange: (u8, u8, u8), violet: (u8, u8, u8), blue: (u8, u8, u8)) -> Color32 {
    let tx = ((pos.x - b.left()) / b.width()).clamp(0.0, 1.0);
    let ty = ((pos.y - b.top()) / b.height()).clamp(0.0, 1.0);
    let t = (tx + ty) / 2.0;
    if t < 0.5 {
        lerp(orange, violet, t * 2.0)
    } else {
        lerp(violet, blue, (t - 0.5) * 2.0)
    }
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}
use nannou::ease;
use nannou::geom::Point2;
use nannou::noise::*;
use nannou::prelude::*;
use nannou::rand::rngs::SmallRng;
use nannou::rand::{Rng, SeedableRng};
mod linspace;
use bspline::BSpline;
use linspace::linspace;

type Noise = Perlin;
const SPLINE_DEGREE: usize = 4;
const OUT_DIR: &str = "frames";
const NFRAMES: usize = 1800;

// see the scipy documentation for the constraints on the number of knots etc.
// https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.BSpline.html
fn set_knots(domain: (f32, f32), degree: usize, npoints: usize) -> Vec<f32> {
    let mut knots = vec![0f32; degree];
    linspace(domain.0, domain.1, npoints - degree).for_each(|v| knots.push(v));
    for _ in 0..(degree + 1) {
        knots.push(domain.1);
    }
    knots
}

fn gen_circle_points(n_control_points: usize, radius: f32) -> Vec<Point2<f32>> {
    let mut shape_points: Vec<_> = geom::Ellipse {
        rect: geom::Rect::from_w_h(radius, radius),
        resolution: n_control_points,
    }
    .circumference()
    .collect();

    // wrap the vector
    let n_wrap = (n_control_points as f32 * 0.3) as usize;
    // NOTE: the `to_vec` makes a copy of the slice which is why this is allowed
    for v in shape_points[..n_wrap].to_vec() {
        shape_points.push(v);
    }

    shape_points
}

fn draw_spline(model: &Model, draw: &Draw) {
    let shape = gen_circle_points(10, model.radius);
    let knots = set_knots((0.0, shape.len() as f32), SPLINE_DEGREE, shape.len());

    let mut rng = model.rng.to_owned();

    for i_line in 0..model.n_lines {
        let mut points = Vec::new(); // NOTE: need to make a new vec each time as it is moved to bspline below
        for p in shape.as_slice() {
            let dx = model.noise.get([
                p.x as f64,
                p.y as f64,
                model.offset + 0.001 * (i_line * 2) as f64,
            ]) as f32;
            let dy = model.noise.get([
                p.x as f64,
                p.y as f64,
                -model.offset + 0.001 * (i_line * 2 + 1) as f64,
            ]) as f32;
            points.push(*p + pt2(dx, dy) * model.magnitude);
        }

        let spline = BSpline::new(SPLINE_DEGREE, points, knots.clone());
        let knot_domain = spline.knot_domain();
        let knot_range = knot_domain.1 - knot_domain.0;

        draw.point_mode()
            .polyline()
            .color(model.color)
            .stroke_weight(0.0)
            .points((0..model.n_grains).map(|p| {
                spline.point((p as f32 / model.n_grains as f32) * knot_range + knot_domain.0)
                    + pt2(rng.gen::<f32>(), rng.gen::<f32>()) * 1.5
            }));
    }
}

struct Model {
    noise: Noise,
    rng: SmallRng,
    radius: f32,
    n_lines: usize,
    n_grains: usize,
    magnitude: f32,
    color: Srgba,
    offset: f64,
}

fn model(app: &App) -> Model {
    let win = app
        .window(app.new_window().view(view).build().unwrap())
        .unwrap();
    app.set_loop_mode(LoopMode::NTimes {
        number_of_updates: NFRAMES,
    });

    let out_dir = std::path::Path::new(OUT_DIR);
    if !out_dir.exists() {
        std::fs::create_dir(out_dir).expect("Failed to create 'frames' directory.");
    }

    let (w, h) = win.rect().w_h();
    let color = rgba(0.0, 0.0, 0.0, 0.01);

    Model {
        noise: Noise::new(),
        rng: SmallRng::seed_from_u64(6382987),
        radius: w.min(h) * 0.4,
        n_lines: 1000,
        n_grains: 8000,
        magnitude: 300.0,
        color,
        offset: 1.0,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let nth = app.elapsed_frames() as f32;
    model.offset = ease::sine::ease_in_out(nth as f64, 1.0, 4.0, NFRAMES as f64);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    frame.clear(rgb8(236, 230, 220));
    draw_spline(model, &draw);

    draw.to_frame(app, &frame).unwrap();
    app.main_window()
        .capture_frame(format!("{}/frame-{:04}.png", OUT_DIR, frame.nth()));
}

fn main() {
    nannou::app(model).update(update).run();
}

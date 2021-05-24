use bspline;
use nannou::color::IntoLinSrgba;
use nannou::draw::properties::ColorScalar;
use nannou::geom::Point2;
use nannou::math::cgmath::Rad;
use nannou::noise::*;
use nannou::prelude::*;
use nannou::rand::rngs::SmallRng;
use nannou::rand::{Rng, SeedableRng};
mod linspace;
use linspace::*;

type Noise = Perlin;
const SPLINE_DEGREE: usize = 4;
const COLORS: [[u8; 3]; 5] = [
    [52, 64, 77],
    [85, 101, 115],
    [171, 185, 201],
    [147, 169, 189],
    [133, 150, 166],
];

fn main() {
    nannou::app(model).update(update).exit(exit).run();
}

struct Model {
    // The noise instance (type is set above)
    noise: Noise,
    // The texture that we will draw to.
    texture: wgpu::Texture,
    // Create a `Draw` instance for drawing to our texture.
    draw: nannou::Draw,
    // The type used to render the `Draw` vertices to our texture.
    renderer: nannou::draw::Renderer,
    // The type used to capture the texture.
    texture_capturer: wgpu::TextureCapturer,
    // The type used to resize our texture to the window texture.
    texture_reshaper: wgpu::TextureReshaper,
    // The RNG (seeded for reproducibility)
    rng: SmallRng,
}

fn model(app: &App) -> Model {
    // Lets write to a 4K UHD texture.
    let texture_size = [2_160, 3_840];

    // Create the window.
    let [win_w, win_h] = [texture_size[0] / 4, texture_size[1] / 4];
    let w_id = app
        .new_window()
        .size(win_w, win_h)
        .title("AEye")
        .view(view)
        .build()
        .unwrap();
    let window = app.window(w_id).unwrap();

    // Retrieve the wgpu device
    let device = window.swap_chain_device();
    // Create our custom texture.
    let sample_count = window.msaa_samples();
    let texture = wgpu::TextureBuilder::new()
        .size(texture_size)
        // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
        // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
        .usage(wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED)
        // Use nannou's default multisampling sample count.
        .sample_count(sample_count)
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing.
        .format(wgpu::TextureFormat::Rgba16Float)
        // Build it!
        .build(device);
    // Create our `Draw` instance and a renderer for it.
    let draw = nannou::Draw::new();
    let descriptor = texture.descriptor();
    let renderer =
        nannou::draw::RendererBuilder::new().build_from_texture_descriptor(device, descriptor);

    // Create the texture capturer.
    let texture_capturer = wgpu::TextureCapturer::default();
    // Create the texture reshaper.
    let texture_view = texture.view().build();
    let texture_sample_type = texture.sample_type();
    let dst_format = Frame::TEXTURE_FORMAT;
    let texture_reshaper = wgpu::TextureReshaper::new(
        device,
        &texture_view,
        sample_count,
        texture_sample_type,
        sample_count,
        dst_format,
    );

    // Set the number of frames which we will generate.
    // I'm doing this because my integrated graphic chip can't handle all the vertices in a single
    // frame render so I have to build things up in multiple frames.
    app.set_loop_mode(LoopMode::NTimes {
        number_of_updates: 202,
    });

    Model {
        noise: Perlin::new(),
        texture: texture,
        draw: draw,
        renderer: renderer,
        texture_capturer: texture_capturer,
        texture_reshaper: texture_reshaper,
        rng: SmallRng::seed_from_u64(38274903),
    }
}

fn gen_circle_points(n_control_points: usize, min_dim: f32, radius_fac: f32) -> Vec<Point2<f32>> {
    let radius = min_dim * radius_fac;
    let mut shape_points: Vec<_> = geom::Ellipse {
        rect: geom::Rect::from_w_h(radius, radius),
        resolution: n_control_points,
    }
    .circumference()
    .collect();

    // wrap the vector
    let n_wrap = (n_control_points as f32 * 0.25) as usize;
    // NB the `to_vec` makes a copy of the slice which is why this is allowed
    for v in shape_points[..n_wrap].to_vec() {
        shape_points.push(v);
    }

    shape_points
}

// see the scipy documentation for the constraints on the number of knots etc.
// https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.BSpline.html
pub fn set_knots(domain: (f32, f32), degree: usize, npoints: usize) -> Vec<f32> {
    let mut knots = vec![0f32; degree];
    linspace(domain.0, domain.1, npoints - degree).for_each(|v| knots.push(v));
    for _ in 0..(degree + 1) {
        knots.push(domain.1);
    }
    knots
}

fn draw_spline<C>(
    model: &mut Model,
    shape_points: Vec<Point2<f32>>,
    n_lines: usize,
    n_grains: usize,
    magnitude: f32,
    color: C,
) where
    C: IntoLinSrgba<ColorScalar> + Copy,
{
    let knots = set_knots(
        (0.0, shape_points.len() as f32),
        SPLINE_DEGREE,
        shape_points.len(),
    );

    for i_line in 0..n_lines {
        let mut points = Vec::new(); // NOTE: need to make a new vec each time as it is moved to bspline below
        for p in shape_points.as_slice() {
            let dx = model
                .noise
                .get([p.x as f64, p.y as f64, 1.0 + 0.002 * (i_line * 2) as f64])
                as f32;
            let dy = model.noise.get([
                p.x as f64,
                p.y as f64,
                -1.0 + 0.002 * (i_line * 2 + 1) as f64,
            ]) as f32;
            points.push(*p + pt2(dx, dy) * magnitude);
        }

        let spline = bspline::BSpline::new(SPLINE_DEGREE, points, knots.clone());
        let knot_domain = spline.knot_domain();
        let knot_range = knot_domain.1 - knot_domain.0;

        model
            .draw
            .point_mode()
            .polyline()
            .color(color)
            .stroke_weight(0.0)
            .points((0..n_grains).map(|p| {
                spline.point((p as f32 / n_grains as f32) * knot_range + knot_domain.0)
                    + pt2(model.rng.gen::<f32>(), model.rng.gen::<f32>()) * 1.5
            }));
    }
}

fn color_to_rgba8(arr: [u8; 3], alpha: f32) -> Rgba8 {
    rgba8(arr[0], arr[1], arr[2], (alpha as f32 * 255.0) as u8)
}

fn update(app: &App, model: &mut Model, _update: Update) {
    // First, reset the `draw` state.
    model.draw.reset();

    // Create a `Rect` for our texture to help with drawing.
    let [w, h] = model.texture.size();
    let r = geom::Rect::from_w_h(w as f32, h as f32);
    let min_dim = r.w().min(r.h());

    let window = app.main_window();
    let nth = window.elapsed_frames();

    match nth {
        0 => {
            {
                let draw = &model.draw;
                draw.background().color(WHITE);

                draw.ellipse()
                    .radius(min_dim * 0.5 * 0.75)
                    .color(color_to_rgba8(COLORS[1], 1.0));
            }

            // outer edge
            draw_spline(
                model,
                gen_circle_points(25, min_dim, 0.75),
                400,
                6000 * 2,
                80.0 * 2.0,
                color_to_rgba8(COLORS[0], 0.02),
            );
        }
        1..=200 => {
            /*
             * Iris
             */

            let theta = Rad(model.rng.gen::<f32>() * TAU);
            let max_r = (0.1 * model.rng.gen::<f32>() + 0.7) * 0.5;
            let n_control_points = (10.0 * model.rng.gen::<f32>()) as usize + 10;
            let color = color_to_rgba8(COLORS[model.rng.gen_range(0, 5) as usize], 0.02);
            draw_spline(
                model,
                linspace(0.1 * min_dim, max_r * min_dim, n_control_points)
                    .map(|r| pt2(r * theta.cos(), r * theta.sin()))
                    .collect(),
                100,
                1000 * 3,
                80.0 * 3.0,
                color,
            );
        }
        201 => {
            /*
             * Pupil
             */
            model
                .draw
                .ellipse()
                .radius(min_dim * 0.5 * 0.25)
                .color(rgb(0.2, 0.2, 0.2));
            draw_spline(
                model,
                gen_circle_points(25, min_dim, 0.25),
                200,
                1000 * 3,
                20.0 * 3.0,
                color_to_rgba8(COLORS[1], 0.02),
            );
            draw_spline(
                model,
                gen_circle_points(8, min_dim, 0.1),
                200 * 2,
                1000 * 2,
                80.0 * 2.0,
                rgba(1.0, 1.0, 1.0, 0.02),
            );
        }
        _ => {}
    };

    // Render our drawing to the texture.
    let device = window.swap_chain_device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("texture renderer"),
    };
    let mut encoder = device.create_command_encoder(&ce_desc);
    model
        .renderer
        .render_to_texture(device, &mut encoder, &model.draw, &model.texture);

    if nth == 201 {
        // Take a snapshot of the texture. The capturer will do the following:
        //
        // 1. Resolve the texture to a non-multisampled texture if necessary.
        // 2. Convert the format to non-linear 8-bit sRGBA ready for image storage.
        // 3. Copy the result to a buffer ready to be mapped for reading.
        let snapshot = model
            .texture_capturer
            .capture(device, &mut encoder, &model.texture);

        // Submit the commands for our drawing **and texture capture** to the GPU.
        window.swap_chain_queue().submit(Some(encoder.finish()));

        // Save the high-res version once we have completed the draw
        println!("nth = {}", nth);
        if nth == 201 {
            snapshot
                .read(move |result| {
                    let image = result.expect("failed to map texture memory").to_owned();
                    image
                        .save("aeye.png")
                        .expect("failed to save texture to png image");
                })
                .unwrap();
        }
    } else {
        // Submit the commands for our drawing to the GPU.
        window.swap_chain_queue().submit(Some(encoder.finish()));
    }
}

fn view(_app: &App, model: &Model, frame: Frame) {
    // Sample the texture and write it to the frame.
    let mut encoder = frame.command_encoder();
    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);
}

// Wait for capture to finish.
fn exit(app: &App, model: Model) {
    println!("Waiting for PNG writing to complete...");
    let window = app.main_window();
    let device = window.swap_chain_device();
    model
        .texture_capturer
        .await_active_snapshots(&device)
        .unwrap();
    println!("Done!");
}

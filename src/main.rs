extern crate num_complex;
extern crate cgmath;
extern crate cairo;
extern crate gio;
extern crate gtk;

use std::env::args;
use std::time::Instant;
use std::rc::Rc;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::DrawingArea;

use std::f32::consts::PI;
use num_complex::Complex32;
use num_complex::Complex;
use cgmath::Quaternion;
use cgmath::Vector3;
use cgmath::InnerSpace;

//const psi: f32  = (5.0_f32.sqrt() - 1.0) / 2.0;
const PSI:  f32 = 0.61803398874989484820;
const PSI2: f32 = 1.0 - PSI; // psi**2 = 1 - psi

#[derive(Copy, Clone, PartialEq)]
enum BTile { L, S }

#[derive(Copy, Clone)]
struct RTile {
    a:      Complex32,
    b:      Complex32,
    c:      Complex32,
    t:      BTile,
}

fn rtile(a: Complex32, b: Complex32, c: Complex32, t: BTile) -> RTile {
    RTile { a: a, b: b, c: c, t: t, }
}

fn reflect(t: RTile) -> RTile {
    RTile { a: t.a.conj(), b: t.b.conj(), c: t.c.conj(), t: t.t }
}

fn inflate_tile(t: RTile) -> Vec<RTile> {
    match t.t {
        BTile::L   => {
            let d = PSI2 * t.a + PSI * t.c;
            let e = PSI2 * t.a + PSI * t.b;
            vec![
                rtile(d, e, t.a, BTile::L),
                rtile(e, d, t.b, BTile::S),
                rtile(t.c, d, t.b, BTile::L),
            ]
        }
        BTile::S   => {
            let d = PSI * t.a + PSI2 * t.b;
            vec![
                rtile(d, t.c, t.a, BTile::S),
                rtile(t.c, d, t.b, BTile::L),
            ]
        }
    }
}

fn inflate(t: Vec<RTile>) -> Vec<RTile> {
    t.iter().flat_map(|x| inflate_tile(*x)).collect()
}

struct Rhomboid {
    a:      Complex32,
    b:      Complex32,
    c:      Complex32,
    d:      Complex32,
    t:      BTile,
}

impl Rhomboid {
    fn center(&self) -> Complex32 {
        (self.a + self.b + self.c + self.d) / 4f32
    }
}

fn tiles_to_rhomboids(mut t: Vec<RTile>) -> Vec<Rhomboid> {
    let mut ret = vec![];

    while !t.is_empty() {
        let x = t.pop().unwrap();

        for i in 0..(t.len()) {
            let y = t[i];

            if x.a == y.a && x.c == y.c {
                t.remove(i);
                ret.push(Rhomboid { a: x.a, b: x.b, c: x.c, d: y.b, t: x.t });
                break;
            }
        }
    }

    ret
}

fn generate_tiling(scale: f32, ngens: usize) -> Vec<RTile> {
    let rot = Complex::from_polar(&1f32, &(PI / 5f32));

    let a1: Complex32 = Complex::new(scale, 0f32);
    let b:  Complex32 = Complex::new(0f32, 0f32);
    let c1 = a1 * rot;
    let a2 = c1 * rot;
    let c3 = a2 * rot;
    let a4 = c3 * rot;
    let c5 = -a1;

    let mut t = vec![
        rtile(a1, b, c1, BTile::S),
        rtile(a2, b, c1, BTile::S),
        rtile(a2, b, c3, BTile::S),
        rtile(a4, b, c3, BTile::S),
        rtile(a4, b, c5, BTile::S),
    ];

    let mut reflected: Vec<RTile> = t.iter()
        .map(|x| reflect(*x))
        .collect();

    t.append(&mut reflected);

    for _ in 0..ngens {
        t = inflate(t);
    }

    t
}

fn rotation_q(v: Vector3<f32>, theta: f32) -> Quaternion<f32> {
    Quaternion::from_sv((theta / 2.0).cos(),
                        v.normalize() * (theta / 2.0).sin())
}

struct PenroseState {
    tiles:      Vec<Rhomboid>,
    beat:       bool,
    bpm:        f32,
    start:      Instant,
}

fn build_ui(application: &gtk::Application) {
    let state = Rc::new(PenroseState {
        tiles:  tiles_to_rhomboids(generate_tiling(10.0, 5)),
        beat:   false,
        bpm:    120f32,
        start:  Instant::now(),
    });

    let window = gtk::ApplicationWindow::new(application);
    window.set_default_size(1000, 1000);

    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);

    let button_beat = gtk::CheckButton::new_with_label("beat");

    button_beat.connect_clicked(move |cr| {
    });
    pane.add(&button_beat);

    let drawing_area = DrawingArea::new();
    let sp = Rc::clone(&state);
    drawing_area.connect_draw(move |_, cr| {
        cr.set_dash(&[3., 2., 1.], 1.);
        cr.scale(500f64, 500f64);

        cr.set_source_rgb(20.0/255.0, 20.0/255.0, 20.0/255.0);
        cr.paint();

        cr.set_line_width(0.0);

        let time = (sp.start.elapsed().as_millis() as f32) / 1000.0;

        let red     = Vector3::new(1f32, 0f32, 0f32);
        let green   = Vector3::new(0f32, 1f32, 0f32);
        let blue    = Vector3::new(0f32, 0f32, 1f32);

        let theta1 = rotation_q(Vector3::new(1f32, 1f32, 1f32), time * 1f32);
        let theta2 = rotation_q(Vector3::new(0f32, 1f32, 0f32), time * 0.2f32);
        let theta3 = rotation_q(Vector3::new(0f32, 0f32, 1f32), time * 0.05f32);

        let color = Quaternion::from_sv(0f32, red) * (theta1 * (theta2 * theta3));

        let beat = time % 0.4;

        for i in sp.tiles.iter() {
            let mut tcolor = color;
            let mut brightness = 1f32;

            if i.t == BTile::L {
                tcolor = tcolor * rotation_q(blue, 0.6f32);
            }

            //tcolor = tcolor * rotation_q(green, i.center().to_polar().1);

            //tcolor *= i.center() / i.center().norm();
            //tcolor *= i.center().norm();
            //tcolor *= time_rot;

            if beat < 100.0 {
                //brightness *= 100.0 / f32::max(1f32, beat);
                brightness *= 10.0 / f32::max(0.1f32, i.center().norm());
            }

            tcolor *= brightness;

            fn colorscale(x: f32) -> f64 {
                ((x + 1.0) / 2.0) as f64
            }

            cr.set_source_rgb(colorscale(tcolor.v[0]),
                              colorscale(tcolor.v[1]),
                              colorscale(tcolor.v[2]));

            fn scale(x: f32) -> f64 {
                ((x / 15.0) + 1.0) as f64
            }

            cr.move_to(scale(i.a.re), scale(i.a.im));
            cr.line_to(scale(i.b.re), scale(i.b.im));
            cr.line_to(scale(i.c.re), scale(i.c.im));
            cr.line_to(scale(i.d.re), scale(i.d.im));
            cr.close_path();
            cr.fill();
        }

        Inhibit(false)
    });
    pane.add2(&drawing_area);

    window.add(&pane);
    window.show_all();

    let tick = move || {
        drawing_area.queue_draw();
        gtk::Continue(true)
    };
    gtk::timeout_add(100, tick);
}

fn main() {
    let application = gtk::Application::new("com.github.gtk-rs.examples.cairotest",
                                            Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

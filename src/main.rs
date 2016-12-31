#[macro_use] extern crate error_chain;
#[macro_use] extern crate nom;
extern crate clap;
extern crate kiss3d;
extern crate nalgebra;
extern crate notify;

mod stl;

mod errors { error_chain! {} }
use errors::*;

use clap::{Arg, App};
use kiss3d::light::Light;
use kiss3d::resource::Mesh;
use kiss3d::window::Window;
use nalgebra::{Point3, Vector3};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

fn main() {
    if let Err(ref e) = run() {
        use ::std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

fn load_file(filename: &str) -> Result<Rc<RefCell<Mesh>>> {
    use std::fs::File;
    let mut fp = File::open(filename).chain_err(|| "unable to open input file")?;
    let stl = stl::Mesh::from_file(&mut fp).chain_err(|| "unable to parse")?;
    println!("Read mesh named: {} with {} tris", stl.name, stl.tris.len());

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for tri in stl.tris {
        let offset = vertices.len() as u32;
        vertices.push(tri.verts[0]);
        vertices.push(tri.verts[1]);
        vertices.push(tri.verts[2]);
        indices.push(Point3::new(offset, offset + 1, offset + 2));
    }
    return Ok(Rc::new(RefCell::new(Mesh::new(vertices, indices, None, None, false))));
}

fn run() -> Result<()> {
    let matches = App::new("My Super Program")
                          .version("1.0")
                          .author("Terrence <terrence.d.cole@gmail.com>")
                          .about("Does awesome things")
                          .arg(Arg::with_name("INPUT")
                               .help("Sets the input file to use")
                               .required(true)
                               .index(1))
                          .get_matches();
    let filename = matches.value_of("INPUT").unwrap();

    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))
        .chain_err(|| "failed to create file watcher")?;
    watcher.watch(filename, RecursiveMode::NonRecursive)
        .chain_err(|| "failed to watch file")?;

    let mut window = Window::new("stl viewer");

    let mut mesh = load_file(filename).chain_err(|| "failed to load stl file")?;
    let mut c = window.add_mesh(mesh, nalgebra::one());
    c.set_color(1.0, 0.0, 0.0);

    window.set_light(Light::StickToCamera);

    while window.render() {
        c.prepend_to_local_rotation(&Vector3::new(0.0f32, 0.014, 0.0));

        match rx.try_recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(_) => {
                        println!("{:?}", event);
                        c.unlink();
                        mesh = load_file(filename).chain_err(|| "failed to load stl file")?;
                        c = window.add_mesh(mesh, nalgebra::one());
                        c.set_color(1.0, 0.0, 0.0);
                    },
                    _ => {}
                }
            }
            Err(_) => {}
        }
    }

    Ok(())
}

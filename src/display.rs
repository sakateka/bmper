extern crate gtk;
extern crate gdk;
extern crate gdk_pixbuf;

use std::io;
use std::process;
use self::gtk::prelude::*;
use self::gtk::{Window, HeaderBar, DrawingArea, WindowType};
use self::gdk::ContextExt;
use self::gdk_pixbuf::Pixbuf;

use pcx;

fn pixbuf_from_file(name: &str) -> Result<Pixbuf, io::Error> {
    let pb = Pixbuf::new_from_file(name);
    if pb.is_ok() {
        return Ok(pb.unwrap());
    }
    if name.to_lowercase().ends_with(".pcx") {
        return pcx::pixbuf_from_file(name);
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Can't load image data: '{}'", pb.unwrap_err()),
    ))
}

pub fn image(name: &str) {
    let image = pixbuf_from_file(name).unwrap_or_else(|e| {
        eprintln!("Can't display: {}", e);
        process::exit(1);
    });

    gtk::init().unwrap_or_else(|e|{
        eprintln!("Failed to initialize GTK Application: {}", e);
        process::exit(1);
    });

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Bmper image display");
    window.set_wmclass("bmper", "Bmper");
    window.connect_delete_event(move |_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let header = HeaderBar::new();
    header.set_title(format!("Display {}", name.split("/").last().unwrap()).as_ref());
    header.set_show_close_button(true);
    window.set_titlebar(&header);

    let height = image.get_height();
    let width = image.get_width();
    window.set_default_size(width, height);

    let draw_area = DrawingArea::new();
    draw_area.connect_draw(move |_, c| {
        c.set_source_pixbuf(&image, 0 as f64, 0 as f64);
        c.paint();
        c.stroke();
        Inhibit(false)
    });
    window.add(&draw_area);
    window.show_all();

    // Start the GTK main event loop
    gtk::main();
}

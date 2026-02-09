//! # Application Driver

use {
    abi::*,
    anyhow::{Result, bail},
    kernel::{
        object::Object,
        shm::{Mutex, SharedMemory},
    },
    std::{
        collections::HashMap,
        sync::atomic::{AtomicU8, AtomicU64, Ordering},
    },
};



fn main() -> Result<()> {
    let mut args = std::env::args();
    let _program_name = args.next();

    let Some(app_name) = args.next() else {
        bail!("no application name provided");
    };

    let fonts = FontsImpl {
        proportional: fontdue::Font::from_bytes(
            epaint_default_fonts::UBUNTU_LIGHT,
            fontdue::FontSettings::default(),
        )
        .map_err(|error| anyhow::anyhow!(error))?,
        cache: HashMap::new(),
    };

    let map = SharedMemory::open(format!("/shmem_{}", app_name).as_str())?;
    let mut map_ptr = map.as_ptr();
    let is_map_initialized: &mut AtomicU8 = unsafe { &mut *(map_ptr as *mut AtomicU8) };
    map_ptr = unsafe { map_ptr.add(size_of::<*mut ()>()) };
    let next_input_id: &mut AtomicU64 = unsafe { &mut *(map_ptr as *mut AtomicU64) };
    map_ptr = unsafe { map_ptr.add(size_of::<*mut ()>()) };

    // Wait for the shell to initialize the map.
    while is_map_initialized.load(Ordering::Relaxed) != 1 {}

    let mutex: Mutex<DriverInput> = unsafe { Mutex::from_existing(map_ptr) }?;

    if app_name == "test" {
        return run_tests(next_input_id, mutex);
    }

    let handle = unsafe { Object::open(format!("/home/{}.so", app_name).as_str()).unwrap() };
    let manifest = handle
        .get::<_, *const abi::Manifest>("__MANIFEST")
        .ok_or(anyhow::anyhow!(
            "Could not find manifest for program '{}'",
            app_name,
        ))?;

    let mut view = abi::View::new(
        ((unsafe { &**manifest }).init)(),
        Box::new(fonts),
        unsafe { &**mutex.lock()? }.known_bounds.size(),
    );

    println!("(driver) Performing initial update...");
    abi::update_pass(&mut view);

    println!("(driver) Performing initial render...");
    abi::render_pass(&mut view, &mut unsafe { &mut **mutex.lock()? }.render);

    println!("(driver) Starting main loop...");

    let mut seen_input_id: u64 = 0;
    'handle_input: loop {
        let input_id = next_input_id.load(Ordering::Relaxed);
        if input_id == seen_input_id {
            continue 'handle_input;
        }
        if input_id == u64::MAX {
            break 'handle_input;
        }

        let mut guard = mutex.lock()?;
        let input = unsafe { &mut **guard };

        seen_input_id = input_id;

        for event in input.drain_events().collect::<Vec<_>>() {
            match event {
                DriverInputEvent::Pointer(pointer_event) => {
                    view.handle_pointer_event(pointer_event);
                    abi::render_pass(&mut view, &mut input.render);
                }
                DriverInputEvent::WindowResize(new_bounds) => {
                    view.resize_window(new_bounds.size());
                    abi::render_pass(&mut view, &mut input.render);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn run_tests(next_input_id: &mut AtomicU64, mutex: Mutex<DriverInput>) -> Result<()> {
    let mut drain_counts = [0; DRIVER_INPUT_EVENT_CAPACITY + 1];
    let mut seen_input_id: u64 = 0;
    'handle_input: loop {
        let input_id = next_input_id.load(Ordering::Relaxed);
        if input_id == seen_input_id {
            continue 'handle_input;
        }
        if input_id == u64::MAX {
            break 'handle_input;
        }

        let mut guard = mutex.lock()?;
        let input = unsafe { &mut **guard };

        seen_input_id = input_id;

        let mut drain_count = 0;
        for _event in input.drain_events() {
            drain_count += 1;
        }

        drain_counts[drain_count] += 1;
    }

    println!("(driver) DRAIN_COUNTS = {drain_counts:?}");

    Ok(())
}



struct FontsImpl {
    proportional: fontdue::Font,
    cache: HashMap<(char, u16), fontdue::Metrics>,
}

impl Fonts for FontsImpl {
    fn measure_text(
        &mut self,
        _id: u64,
        text: &str,
        _max_advance: Option<f32>,
        font_size: f32,
        _line_height: LineHeight,
        _font_style: FontStyle,
        _alignment: TextAlignment,
        _wrap_mode: TextWrapMode,
    ) -> Xy<f32> {
        // let mut min_y = f32::MAX;
        // let mut max_y = f32::MIN;
        let line_metrics = self
            .proportional
            .horizontal_line_metrics(font_size)
            .unwrap();
        let width = text.chars().fold(0.0, |acc, ch| {
            let entry = self
                .cache
                .entry((ch, font_size as u16))
                .or_insert_with(|| self.proportional.metrics(ch, font_size));
            // min_y = min_y.min(entry.ymin as f32);
            // max_y = max_y.max(entry.height as f32 + entry.ymin as f32);

            acc + entry.advance_width as f32
        });

        Xy::new(width, line_metrics.new_line_size)
    }
}



#[allow(unused)]
#[unsafe(export_name = "__ui_Label__children_ids")]
pub extern "Rust" fn __label_children_ids(_label: &Label) -> Vec<u64> {
    Vec::new()
}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__render")]
pub extern "Rust" fn __label_render(label: &mut Label, pass: &mut RenderPass<'_>) {
    pass.fill_quad(
        pass.bounds(),
        Rgba {
            r: 11,
            g: 11,
            b: 11,
            a: 255,
        },
        0.0,
        Rgba::NONE,
    );
    pass.fill_text(
        &label.text,
        pass.bounds(),
        Rgba {
            r: 177,
            g: 177,
            b: 177,
            a: 255,
        },
        label.font_size,
    );
}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__layout")]
pub extern "Rust" fn __label_layout(_label: &mut Label, _pass: &mut LayoutPass<'_>) {}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__measure")]
pub extern "Rust" fn __label_measure(
    label: &mut Label,
    context: &mut MeasureContext<'_>,
    axis: Axis,
    length_request: LengthRequest,
    cross_length: Option<f32>,
) -> f32 {
    let id = context.id();
    let fonts = context.fonts_mut();
    // For exact measurements, we round up so the `FontsImpl` doesn't wrap
    // unnecessarily.
    let max_advance = match axis {
        Axis::Horizontal => match length_request {
            LengthRequest::MinContent => Some(0.0),
            LengthRequest::MaxContent => None,
            LengthRequest::FitContent(space) => Some((space + 0.5).round()),
        },
        Axis::Vertical => match length_request {
            LengthRequest::MinContent => cross_length.or(Some(0.0)),
            LengthRequest::MaxContent | LengthRequest::FitContent(_) => {
                cross_length.map(|l| (l + 0.5).round())
            }
        },
    };
    let used_size = fonts.measure_text(
        id,
        &label.text,
        max_advance,
        label.font_size,
        label.line_height,
        label.font_style,
        label.alignment,
        label.wrap_mode,
    );

    used_size.value_for_axis(axis)
}

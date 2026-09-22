use crate::app::App;
use crate::engine::canvas::CanvasRenderer;
use crate::engine::input::{InputState, KeyCode};
use crate::engine::render::DrawList;
use crate::engine::GameDriver;
use crate::game::constants::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const STEP: f64 = 1.0 / 60.0;
const MAX_FRAME: f64 = 0.25;

/// Wasm entry point. Call after the page's canvas is in the DOM.
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into()?;
    canvas.set_width(ARENA_W as u32);
    canvas.set_height(ARENA_H as u32);

    let context: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?
        .dyn_into()?;

    let input = Rc::new(RefCell::new(InputState::new()));
    let performance = window.performance().ok_or_else(|| JsValue::from_str("no performance"))?;
    let app = Rc::new(RefCell::new(App::new_seeded(performance.now() as u32)));
    let renderer = Rc::new(RefCell::new(CanvasRenderer::new(context, ARENA_W, ARENA_H)));
    let list = Rc::new(RefCell::new(DrawList::new()));

    let input_for_keys = Rc::clone(&input);
    let on_key_down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if let Some(code) = KeyCode::from_name(&event.key()) {
            event.prevent_default();
            input_for_keys.borrow_mut().key_down(code);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    window.add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())?;
    on_key_down.forget();

    let input_for_keys = Rc::clone(&input);
    let on_key_up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if let Some(code) = KeyCode::from_name(&event.key()) {
            input_for_keys.borrow_mut().key_up(code);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    window.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())?;
    on_key_up.forget();

    let loop_handle: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let loop_ref = Rc::clone(&loop_handle);

    let window_loop = window.clone();
    let performance_loop = performance.clone();
    let input_loop = Rc::clone(&input);
    let app_loop = Rc::clone(&app);
    let renderer_loop = Rc::clone(&renderer);
    let list_loop = Rc::clone(&list);
    let accumulator = Rc::new(RefCell::new(0.0f64));
    let last_tick = Rc::new(RefCell::new(performance.now()));

    *loop_handle.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let now = performance_loop.now();
        let dt = ((now - *last_tick.borrow()) / 1000.0).min(MAX_FRAME);
        *last_tick.borrow_mut() = now;
        *accumulator.borrow_mut() += dt;

        while *accumulator.borrow() >= STEP {
            let frame = input_loop.borrow_mut().begin_frame();
            app_loop
                .borrow_mut()
                .simulate(STEP as f32, &input_loop.borrow(), &frame);
            *accumulator.borrow_mut() -= STEP;
        }

        let a = app_loop.borrow();
        a.draw(&mut *list_loop.borrow_mut());
        drop(a);
        renderer_loop.borrow_mut().render(&*list_loop.borrow());

        if let Some(f) = loop_ref.borrow().as_ref() {
            let _ = window_loop.request_animation_frame(f.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));

    let binding = loop_handle.borrow();
    let first = binding.as_ref().unwrap().as_ref().unchecked_ref();
    window.request_animation_frame(first)?;
    Ok(())
}
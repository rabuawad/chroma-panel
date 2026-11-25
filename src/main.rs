mod desktop;

use gpui::{prelude::*, *};
use gpui_platform::application;
use image::{Frame, RgbaImage};
use std::{cell::Cell, rc::Rc, sync::Arc};

const MODES: [&str; 3] = ["Grayscale", "Sepia", "Invert"];
const ACCENT: u32 = 0xa64223;
const INK: u32 = 0x302b28;

fn preview_images() -> Vec<Arc<RenderImage>> {
    let base = image::load_from_memory(include_bytes!("../assets/pencils.png"))
        .unwrap()
        .to_rgba8();
    (0..4)
        .map(|index| {
            let pixels = RgbaImage::from_fn(base.width(), base.height(), |x, y| {
                let [r, g, b, _] = base.get_pixel(x, y).0.map(f32::from);
                let rgb = match index {
                    1 => [0.2126 * r + 0.7152 * g + 0.0722 * b; 3],
                    2 => [
                        0.393 * r + 0.769 * g + 0.189 * b,
                        0.349 * r + 0.686 * g + 0.168 * b,
                        0.272 * r + 0.534 * g + 0.131 * b,
                    ],
                    3 => [255. - r, 255. - g, 255. - b],
                    _ => [r, g, b],
                };
                image::Rgba([
                    rgb[2].round() as u8,
                    rgb[1].round() as u8,
                    rgb[0].round() as u8,
                    255,
                ])
            });
            Arc::new(RenderImage::new([Frame::new(pixels)]))
        })
        .collect()
}

struct Panel {
    enabled: bool,
    mode: u32,
    intensity: u32,
    desktop: Option<zbus::blocking::Connection>,
    images: Vec<Arc<RenderImage>>,
    track: Rc<Cell<Bounds<Pixels>>>,
    dragging: bool,
    focus: [FocusHandle; 5],
}

impl Panel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (desktop, (enabled, mode, intensity)) = desktop::connect()
            .map(|(bus, state)| (Some(bus), state))
            .unwrap_or((None, (false, 0, 100)));
        let focus = std::array::from_fn(|i| cx.focus_handle().tab_stop(true).tab_index(i as isize));
        window.focus(&focus[0], cx);
        Self {
            enabled,
            mode,
            intensity,
            desktop,
            images: preview_images(),
            track: Rc::default(),
            dragging: false,
            focus,
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        if let Some(bus) = &self.desktop {
            if let Err(err) = desktop::apply(bus, (self.enabled, self.mode, self.intensity)) {
                eprintln!("Desktop filter disconnected: {err}");
                self.desktop = None;
            }
        }
        cx.notify();
    }

    fn activate(&mut self, i: usize, cx: &mut Context<Self>) {
        match i {
            0 => self.enabled = !self.enabled,
            1..=3 => self.mode = (i - 1) as u32,
            _ => return,
        }
        self.changed(cx);
    }

    fn set_intensity(&mut self, level: u32, cx: &mut Context<Self>) {
        if self.intensity != level {
            self.intensity = level;
            self.changed(cx);
        }
    }

    fn slide(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let b = self.track.get();
        let level = ((x - b.origin.x) / b.size.width * 100.)
            .round()
            .clamp(0., 100.) as u32;
        self.set_intensity(level, cx);
    }

    fn control(&self, i: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(("control", i))
            .track_focus(&self.focus[i])
            .cursor_pointer()
            .border_2()
            .border_color(transparent_black())
            .focus_visible(|s| s.border_color(rgb(ACCENT)))
            .on_click(cx.listener(move |v, _, w, cx| {
                w.focus(&v.focus[i], cx);
                v.activate(i, cx);
            }))
    }
}

fn row() -> Div {
    div().flex().items_center().justify_between().px_5().py_3()
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let level = self.intensity as f32 / 100.;
        let track = self.track.clone();
        let increment = cx.entity().downgrade();
        let decrement = increment.clone();
        let slider = self
            .control(4, cx)
            .role(Role::Slider)
            .aria_label("Intensity")
            .aria_numeric_value(self.intensity.into())
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(100.)
            .aria_numeric_value_step(1.)
            .aria_orientation(Orientation::Horizontal)
            .on_a11y_action(AccessibleAction::Increment, move |_, _, cx| {
                let _ =
                    increment.update(cx, |v, cx| v.set_intensity((v.intensity + 1).min(100), cx));
            })
            .on_a11y_action(AccessibleAction::Decrement, move |_, _, cx| {
                let _ = decrement.update(cx, |v, cx| {
                    v.set_intensity(v.intensity.saturating_sub(1), cx)
                });
            })
            .h(px(34.))
            .rounded_md()
            .px_2()
            .flex()
            .items_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|v, e: &MouseDownEvent, w, cx| {
                    w.focus(&v.focus[4], cx);
                    v.dragging = true;
                    v.slide(e.position.x, cx);
                }),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(0xe3dcd5))
                    .child(
                        canvas(move |b, _, _| track.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    )
                    .child(
                        div()
                            .h_full()
                            .w(relative(level))
                            .rounded_full()
                            .bg(rgb(ACCENT)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(relative(level))
                            .ml(px(-8.))
                            .top(px(-5.))
                            .size(px(16.))
                            .rounded_full()
                            .bg(white())
                            .border_2()
                            .border_color(rgb(ACCENT)),
                    ),
            );
        let mut modes = div().flex().gap_1().items_center();
        for (i, name) in MODES.iter().enumerate() {
            let selected = self.mode == i as u32;
            modes = modes.child(
                self.control(i + 1, cx)
                    .role(Role::Button)
                    .aria_label(*name)
                    .aria_toggled(if selected {
                        Toggled::True
                    } else {
                        Toggled::False
                    })
                    .rounded_md()
                    .px_3()
                    .py_1()
                    .text_sm()
                    .bg(rgb(if selected { 0xf6e1d6 } else { 0xf5f3f1 }))
                    .text_color(rgb(if selected { 0x843317 } else { INK }))
                    .child(*name),
            );
        }
        let scope = if self.desktop.is_some() {
            "Desktop filter · GNOME"
        } else {
            "Preview only · Enable the GNOME extension for desktop filtering"
        };
        div().id("panel").size_full().overflow_y_scroll().bg(rgb(0xf8f6f3)).text_color(rgb(INK)).font_family("Ubuntu")
            .on_key_down(cx.listener(|v,e:&KeyDownEvent,w,cx| {
                let key = e.keystroke.key.as_str();
                if key=="tab" { if e.keystroke.modifiers.shift { w.focus_prev(cx); } else { w.focus_next(cx); } return; }
                let Some(i)=v.focus.iter().position(|f| f.is_focused(w)) else{return};
                if matches!(key,"space"|"enter") {v.activate(i,cx);}
                if i==4 {
                    let n=match key {"left"|"down"=>v.intensity.saturating_sub(1),"right"|"up"=>(v.intensity+1).min(100),"home"=>0,"end"=>100,_=>return};
                    v.set_intensity(n,cx);
                }
            }))
            .on_mouse_move(cx.listener(|v,e:&MouseMoveEvent,_,cx|{
                if v.dragging && e.pressed_button==Some(MouseButton::Left) {v.slide(e.position.x,cx);} else {v.dragging=false;}
            }))
            .on_mouse_up(MouseButton::Left,cx.listener(|v,_,_,_| v.dragging=false))
            .child(div().p_6().flex().flex_col().gap_4()
                .child(div().child(div().text_3xl().font_weight(FontWeight::BOLD).child("Color filters"))
                    .child(div().text_sm().text_color(rgb(0x645b55)).child(scope)))
                .child(div().bg(white()).rounded_xl().overflow_hidden().border_1().border_color(rgb(0xded9d4))
                    .child(div().relative().bg(rgb(0xf2f2f2)).w_full().h(px(205.))
                        .child(img(self.images[0].clone()).size_full().object_fit(ObjectFit::Contain))
                        .when(self.desktop.is_none() && self.enabled, |s| s.child(div().absolute().top_0().left_0().size_full().opacity(level)
                            .child(img(self.images[1+self.mode as usize].clone()).size_full().object_fit(ObjectFit::Contain)))))
                    .child(row().child("Color filters").child(self.control(0,cx).role(Role::Switch).aria_label("Color filters")
                        .aria_toggled(if self.enabled {Toggled::True} else {Toggled::False})
                        .w(px(48.)).h(px(28.)).flex().items_center().px(px(2.)).rounded_full()
                        .bg(rgb(if self.enabled {ACCENT} else {0x827c76})).when(self.enabled,|s| s.justify_end())
                        .child(div().size(px(20.)).rounded_full().bg(white()))))
                    .child(row().border_t_1().border_color(rgb(0xeae5df)).child("Filter type").child(modes)))
                .child(div().p_4().rounded_xl().bg(white()).border_1().border_color(rgb(0xded9d4))
                    .child(div().flex().justify_between().child("Intensity").child(format!("{}%",self.intensity)))
                    .child(slider)
                    .child(div().flex().justify_between().px_2().text_xs().text_color(rgb(0x645b55)).child("Low").child("High"))))
    }
}

fn main() {
    application().run(|cx| {
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(660.), px(566.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(460.), px(350.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Chroma Panel".into()),
                    ..Default::default()
                }),
                app_id: Some("local.ChromaPanel".into()),
                ..Default::default()
            },
            |w, cx| cx.new(|cx| Panel::new(w, cx)),
        )
        .expect("open the color filters window");
        cx.activate(true);
    });
}

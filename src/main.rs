mod colors;
mod desktop;
mod icons;
mod preview;
use icons::{Icon, icon};

use colors::{Color, Library, PRESETS, channels};
use gpui::{prelude::*, *};
use gpui_platform::application;
use image::{Frame, RgbaImage};
use std::{cell::Cell, rc::Rc, sync::Arc};

const BASE: u32 = 0x191523;
const CARD: u32 = 0x211c2c;
const LINE: u32 = 0x42384f;
const TEXT: u32 = 0xf5efff;
const MUTED: u32 = 0xc5b9d2;
const ACCENT: u32 = 0x914cef;
const INTENSITY_STEP: u32 = 10;
const MODES: [&str; 4] = ["Grayscale", "Sepia", "Invert", "Tint"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Track {
    Field,
    Hue,
    Intensity,
    Channel(usize),
}
impl Track {
    fn index(self) -> usize {
        match self {
            Self::Field => 0,
            Self::Hue => 1,
            Self::Intensity => 2,
            Self::Channel(i) => 3 + i,
        }
    }
}

struct Panel {
    root_focus: FocusHandle,
    color: Color,
    library: Library,
    filter: desktop::Filter,
    initial: desktop::Filter,
    bus: Option<desktop::Desktop>,
    tab: usize,
    show_desktop_preview: bool,
    preview: preview::DesktopPreview,
    menu: bool,
    expanded: [bool; 3],
    drag: Option<Track>,
    bounds: [Rc<Cell<Bounds<Pixels>>>; 6],
    field: Option<((u32, u32, u32, u32), Arc<RenderImage>)>,
    message: Option<String>,
}

fn surface() -> Div {
    div()
        .rounded_lg()
        .border_1()
        .border_color(rgb(LINE))
        .bg(rgb(CARD))
}
fn row() -> Div {
    div().flex().items_center().justify_between().gap_2()
}
fn caption(label: impl Into<SharedString>) -> Div {
    div().text_sm().text_color(rgb(MUTED)).child(label.into())
}
fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .focusable()
        .tab_stop(true)
        .aria_label(label.into())
        .rounded_md()
        .border_1()
        .border_color(rgb(LINE))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .hover(|s| s.border_color(rgb(0xb79bda)))
        .focus_visible(|s| s.border_color(white()))
}
fn tag(label: impl Into<SharedString>) -> Div {
    surface()
        .px_2()
        .py_1()
        .text_sm()
        .font_family("monospace")
        .child(label.into())
}
fn gradient(a: u32, b: u32) -> Background {
    linear_gradient(
        180.,
        linear_color_stop(rgb(a), 0.),
        linear_color_stop(rgb(b), 1.),
    )
}

impl Panel {
    fn new(root_focus: FocusHandle) -> Self {
        let library = Library::load();
        let (bus, filter) = desktop::Desktop::connect(library.selected)
            .map(|(b, s)| (Some(b), s))
            .unwrap_or((None, (false, desktop::TINT, 100, library.selected)));
        Self {
            root_focus,
            color: Color::from_rgb(if filter.1 == desktop::TINT {
                filter.3
            } else {
                library.selected
            }),
            library,
            filter,
            initial: filter,
            bus,
            tab: 0,
            show_desktop_preview: false,
            preview: Default::default(),
            menu: false,
            expanded: [true; 3],
            drag: None,
            bounds: Default::default(),
            field: None,
            message: None,
        }
    }
    fn apply(&mut self, cx: &mut Context<Self>) {
        if let Some(bus) = &self.bus
            && let Err(err) = bus.apply(self.filter)
        {
            eprintln!("Desktop filter disconnected: {err}");
            self.bus = None;
        }
        cx.notify();
    }
    fn picked(&mut self, cx: &mut Context<Self>) {
        self.filter.1 = desktop::TINT;
        self.filter.3 = self.color.hex();
        self.apply(cx);
    }
    fn select(&mut self, hex: u32, cx: &mut Context<Self>) {
        self.color = Color::from_rgb(hex);
        self.library.remember(hex);
        self.message = None;
        self.picked(cx);
    }
    fn adjust(&mut self, track: Track, x: f32, y: f32, cx: &mut Context<Self>) {
        self.message = None;
        match track {
            Track::Field => {
                self.color.s = x;
                self.color.v = 1. - y;
            }
            Track::Hue => self.color.h = y,
            Track::Intensity => {
                let n = (x * (100 / INTENSITY_STEP) as f32).round() as u32 * INTENSITY_STEP;
                if self.filter.2 != n {
                    self.filter.2 = n;
                    self.apply(cx);
                }
                return;
            }
            Track::Channel(i) => {
                let mut rgb = channels(self.color.hex());
                rgb[i] = (255. * x).round() as u8;
                self.color = Color::from_rgb(u32::from_be_bytes([0, rgb[0], rgb[1], rgb[2]]));
            }
        }
        self.picked(cx);
    }
    fn pointer(&mut self, t: Track, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let b = self.bounds[t.index()].get();
        if b.size.width > px(0.) && b.size.height > px(0.) {
            self.adjust(
                t,
                ((pos.x - b.origin.x) / b.size.width).clamp(0., 1.),
                ((pos.y - b.origin.y) / b.size.height).clamp(0., 1.),
                cx,
            );
        }
    }
    fn keyboard(&mut self, t: Track, key: &str, cx: &mut Context<Self>) {
        let d = match key {
            "left" | "down" => -1.,
            "right" | "up" => 1.,
            "home" | "end" => 0.,
            _ => return,
        };
        let change = |v: f32, step: f32| match key {
            "home" => 0.,
            "end" => 1.,
            _ => (v + d * step).clamp(0., 1.),
        };
        match t {
            Track::Field => {
                let mut s = self.color.s;
                let mut v = self.color.v;
                if matches!(key, "up" | "down") {
                    v = change(v, 0.01)
                } else {
                    s = change(s, 0.01)
                }
                self.adjust(t, s, 1. - v, cx);
            }
            Track::Hue => self.adjust(t, 0., change(self.color.h, 1. / 360.), cx),
            Track::Intensity => {
                let current = self.filter.2;
                let next = match key {
                    "home" => 0,
                    "end" => 100,
                    "left" | "down" => current.saturating_sub(1) / INTENSITY_STEP * INTENSITY_STEP,
                    _ => ((current / INTENSITY_STEP + 1) * INTENSITY_STEP).min(100),
                };
                self.adjust(t, next as f32 / 100., 0., cx);
            }
            Track::Channel(i) => self.adjust(
                t,
                change(channels(self.color.hex())[i] as f32 / 255., 1. / 255.),
                0.,
                cx,
            ),
        }
    }
    fn track(&self, t: Track, label: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let bounds = self.bounds[t.index()].clone();
        let inc = cx.entity().downgrade();
        let dec = inc.clone();
        let (value, max) = match t {
            Track::Field => (self.color.s as f64 * 100., 100.),
            Track::Hue => (self.color.h as f64 * 360., 360.),
            Track::Intensity => (self.filter.2 as f64, 100.),
            Track::Channel(i) => (channels(self.color.hex())[i] as f64, 255.),
        };
        div()
            .id(("track", t.index()))
            .focusable()
            .tab_stop(true)
            .role(Role::Slider)
            .aria_label(label)
            .aria_numeric_value(value.round())
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(max)
            .aria_numeric_value_step(if t == Track::Intensity {
                INTENSITY_STEP.into()
            } else {
                1.
            })
            .aria_orientation(if t == Track::Hue {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .on_a11y_action(AccessibleAction::Increment, move |_, _, cx| {
                let _ = inc.update(cx, |v, cx| v.keyboard(t, "right", cx));
            })
            .on_a11y_action(AccessibleAction::Decrement, move |_, _, cx| {
                let _ = dec.update(cx, |v, cx| v.keyboard(t, "left", cx));
            })
            .relative()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(|s| s.border_color(white()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, e: &MouseDownEvent, _, cx| {
                    v.drag = Some(t);
                    v.pointer(t, e.position, cx);
                }),
            )
            .on_key_down(
                cx.listener(move |v, e: &KeyDownEvent, _, cx| v.keyboard(t, &e.keystroke.key, cx)),
            )
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
    }
    fn legacy_tint(&self) -> bool {
        self.filter.1 == desktop::TINT && self.bus.as_ref().is_some_and(|b| !b.supports_tint)
    }
    fn field_image(&mut self) -> Arc<RenderImage> {
        let mode = if self.filter.0 && (self.bus.is_none() || self.legacy_tint()) {
            self.filter.1 + 1
        } else {
            0
        };
        let key = (
            self.color.h.to_bits(),
            mode,
            if mode == 0 { 0 } else { self.filter.2 },
            if mode == 4 { self.filter.3 } else { 0 },
        );
        if let Some((prev, image)) = &self.field
            && *prev == key
        {
            return image.clone();
        }
        let pixels = RgbaImage::from_fn(256, 192, |x, y| {
            let c = Color {
                h: self.color.h,
                s: x as f32 / 255.,
                v: 1. - y as f32 / 191.,
            };
            let [r, g, b] = channels(c.hex()).map(f32::from);
            let target = match mode {
                1 => [0.2126 * r + 0.7152 * g + 0.0722 * b; 3],
                2 => [
                    0.393 * r + 0.769 * g + 0.189 * b,
                    0.349 * r + 0.686 * g + 0.168 * b,
                    0.272 * r + 0.534 * g + 0.131 * b,
                ],
                3 => [255. - r, 255. - g, 255. - b],
                4 => {
                    let tint = channels(key.3).map(preview::tint_multiplier);
                    [r * tint[0], g * tint[1], b * tint[2]]
                }
                _ => [r, g, b],
            };
            let amount = key.2 as f32 / 100.;
            let mix = |i: usize, n: f32| (n + (target[i].min(255.) - n) * amount).round() as u8;
            image::Rgba([mix(2, b), mix(1, g), mix(0, r), 255])
        });
        let image = Arc::new(RenderImage::new([Frame::new(pixels)]));
        self.field = Some((key, image.clone()));
        image
    }
    fn picker(&mut self, height: Pixels, cx: &mut Context<Self>) -> Div {
        let image = self.field_image();
        let field = self
            .track(Track::Field, "Saturation and brightness", cx)
            .flex_1()
            .h_full()
            .rounded_lg()
            .overflow_hidden()
            .child(
                img(image)
                    .rounded_lg()
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(self.color.s))
                    .top(relative(1. - self.color.v))
                    .ml(px(-9.))
                    .mt(px(-9.))
                    .size(px(18.))
                    .rounded_full()
                    .border_2()
                    .border_color(white())
                    .shadow_md(),
            );
        let mut hue = self
            .track(Track::Hue, "Hue", cx)
            .w(px(28.))
            .h_full()
            .flex()
            .flex_col()
            .rounded_lg()
            .overflow_hidden();
        for i in 0..6 {
            let hue_at = |n| {
                Color {
                    h: n as f32 / 6.,
                    s: 1.,
                    v: 1.,
                }
                .hex()
            };
            hue = hue.child(
                div()
                    .flex_1()
                    .w_full()
                    .bg(gradient(hue_at(i), hue_at(i + 1))),
            );
        }
        hue = hue.child(
            div()
                .absolute()
                .top(relative(self.color.h.clamp(0.015, 0.985)))
                .mt(px(-4.))
                .w_full()
                .h(px(8.))
                .rounded_full()
                .border_2()
                .border_color(white())
                .bg(rgb(self.color.hex())),
        );
        div()
            .w_full()
            .h(height)
            .flex()
            .gap_3()
            .child(field)
            .child(hue)
    }
    fn color_box(&mut self, cx: &mut Context<Self>) -> Div {
        if self.filter.1 != desktop::TINT {
            return self.picker(px(305.), cx);
        }
        let mut tabs = div()
            .id("tint-tabs")
            .role(Role::TabList)
            .aria_label("Tint view")
            .flex()
            .gap_1();
        for (index, label) in ["Color", "Desktop preview"].into_iter().enumerate() {
            let preview = index == 1;
            let selected = preview == self.show_desktop_preview;
            tabs = tabs.child(
                button(("tint-tab", index), label)
                    .role(Role::Tab)
                    .aria_selected(selected)
                    .h(px(30.))
                    .px_3()
                    .text_sm()
                    .child(label)
                    .when(selected, |s| {
                        s.bg(rgb(0x382845)).border_color(rgb(0xa775da))
                    })
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.show_desktop_preview = preview;
                        cx.notify();
                    })),
            );
        }
        let height = px(267.);
        let content = if self.show_desktop_preview {
            // An active compositor filter already colors the entire window.
            let live = self.filter.0 && self.bus.as_ref().is_some_and(|bus| bus.supports_tint);
            let intensity = if live { 0 } else { self.filter.2 };
            let image = self.preview.image(self.filter.3, intensity);
            div()
                .id("mock-desktop")
                .relative()
                .overflow_hidden()
                .flex()
                .justify_center()
                .items_center()
                .role(Role::Image)
                .aria_label(format!(
                    "Mock Ubuntu desktop with #{:06X} tint at {}%",
                    self.filter.3, self.filter.2
                ))
                .w_full()
                .h(height)
                .bg(rgb(BASE))
                .rounded_lg()
                .child(
                    img(image)
                        .w(height * 2.)
                        .max_w_full()
                        .h(height)
                        .object_fit(ObjectFit::Contain)
                        .rounded_lg(),
                )
                .into_any_element()
        } else {
            self.picker(height, cx).into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(tabs)
            .child(content)
    }

    fn bar(
        &self,
        t: Track,
        label: &'static str,
        fraction: f32,
        color: u32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.track(t, label, cx)
            .w_full()
            .h(px(30.))
            .flex()
            .items_center()
            .cursor_pointer()
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(6.))
                    .bg(rgb(0x494050))
                    .rounded_full()
                    .child(
                        div()
                            .h_full()
                            .w(relative(fraction))
                            .rounded_full()
                            .bg(rgb(color)),
                    )
                    .when(t == Track::Intensity, |rail| {
                        rail.children((0..=100).step_by(INTENSITY_STEP as usize).map(|percent| {
                            div()
                                .absolute()
                                .left(relative(percent as f32 / 100.))
                                .ml(px(-1.))
                                .top(px(-2.))
                                .w(px(2.))
                                .h(px(10.))
                                .rounded_full()
                                .bg(rgb(0xcbbad8))
                        }))
                    })
                    .child(
                        div()
                            .absolute()
                            .left(relative(fraction))
                            .ml(px(-5.))
                            .top(px(-5.))
                            .w(px(10.))
                            .h(px(16.))
                            .rounded_md()
                            .bg(rgb(0xe5cfff))
                            .border_1()
                            .border_color(white()),
                    ),
            )
    }
    fn readouts(&self, cx: &mut Context<Self>) -> Div {
        let hex = self.color.hex();
        let [r, g, b] = channels(hex);
        let [h, s, l] = self.color.hsl();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .size(px(32.))
                    .rounded_md()
                    .bg(rgb(hex))
                    .border_1()
                    .border_color(rgb(LINE)),
            )
            .child(
                button("copy", "Copy hex color")
                    .h(px(32.))
                    .px_2()
                    .bg(rgb(CARD))
                    .text_sm()
                    .font_family("monospace")
                    .child(format!("#{hex:06X}  ⧉"))
                    .on_click(cx.listener(|v, _, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(format!(
                            "#{:06X}",
                            v.color.hex()
                        )));
                        v.message = Some("Hex copied".into());
                        cx.notify();
                    })),
            )
            .child(tag(format!("R {r}  G {g}  B {b}")))
            .child(tag(format!("H {h}°  S {s}%  L {l}%")))
    }
    fn filters(&self, cx: &mut Context<Self>) -> Div {
        let line = || row().min_h(px(54.)).border_t_1().border_color(rgb(LINE));
        let on = self.filter.0;
        let switch = button("switch", "Color filters")
            .role(Role::Switch)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .w(px(42.))
            .h(px(24.))
            .px(px(2.))
            .rounded_full()
            .bg(rgb(if on { ACCENT } else { 0x453a50 }))
            .when(on, |s| s.justify_end())
            .when(!on, |s| s.justify_start())
            .child(div().size(px(18.)).rounded_full().bg(rgb(TEXT)))
            .on_click(cx.listener(|v, _, _, cx| {
                v.filter.0 = !v.filter.0;
                v.apply(cx);
            }));
        let modes = button("mode", "Choose filter type")
            .aria_expanded(self.menu)
            .h(px(32.))
            .px_3()
            .bg(rgb(CARD))
            .child(format!("{}   ⌄", MODES[self.filter.1 as usize]))
            .on_click(cx.listener(|v, _, _, cx| {
                v.menu = !v.menu;
                cx.notify();
            }));
        let mut group = div()
            .flex()
            .flex_col()
            .child(
                line()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(icon(Icon::Filters))
                            .child("Color filters"),
                    )
                    .child(switch),
            )
            .child(
                line()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(icon(Icon::Sliders))
                            .child("Filter type"),
                    )
                    .child(modes),
            );
        if self.menu {
            let mut options = div().flex().justify_end().gap_1().pb_2();
            for (i, name) in MODES.iter().enumerate() {
                options = options.child(
                    button(("mode", i), *name)
                        .px_2()
                        .py_1()
                        .child(*name)
                        .on_click(cx.listener(move |v, _, _, cx| {
                            v.filter.1 = i as u32;
                            if v.filter.1 == desktop::TINT {
                                v.filter.3 = v.color.hex();
                            }
                            v.menu = false;
                            v.apply(cx);
                        })),
                );
            }
            group = group.child(options);
        }
        group.child(
            line()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(icon(Icon::Sun))
                        .child("Intensity"),
                )
                .child(
                    div()
                        .w(relative(0.66))
                        .pr_2()
                        .pt_2()
                        .child(self.bar(
                            Track::Intensity,
                            "Intensity",
                            self.filter.2 as f32 / 100.,
                            ACCENT,
                            cx,
                        ))
                        .child(
                            row()
                                .child(caption("Low"))
                                .child(caption(format!("{}%", self.filter.2)))
                                .child(caption("High")),
                        ),
                ),
        )
    }
    fn swatch(&self, id: ElementId, hex: u32, cx: &mut Context<Self>) -> Stateful<Div> {
        button(id, format!("Select #{hex:06X}"))
            .w(relative(0.125))
            .h(px(38.))
            .p(px(2.))
            .border_color(transparent_black())
            .child(
                div()
                    .size_full()
                    .rounded_md()
                    .bg(rgb(hex))
                    .border_2()
                    .border_color(if self.color.hex() == hex {
                        white()
                    } else {
                        transparent_black()
                    }),
            )
            .on_click(cx.listener(move |v, _, _, cx| v.select(hex, cx)))
    }
    fn group(&self, i: usize, title: &'static str, body: Div, cx: &mut Context<Self>) -> Div {
        let mut header = row().px_2().h(px(38.)).child(
            button(("section", i), title)
                .aria_expanded(self.expanded[i])
                .border_color(transparent_black())
                .px_2()
                .h_full()
                .child(format!(
                    "{}   {title}",
                    if self.expanded[i] { "⌄" } else { "›" }
                ))
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.expanded[i] = !v.expanded[i];
                    cx.notify();
                })),
        );
        if i == 0 {
            header = header.child(
                button("add-preset", "Add current color to presets")
                    .size(px(26.))
                    .child("+")
                    .on_click(cx.listener(|v, _, _, cx| {
                        let c = v.color.hex();
                        if !PRESETS.contains(&c) && !v.library.custom.contains(&c) {
                            v.library.custom.push(c);
                        }
                        v.expanded[0] = true;
                        cx.notify();
                    })),
            );
        }
        surface().child(header).when(self.expanded[i], |s| {
            s.child(div().border_t_1().border_color(rgb(LINE)).p_2().child(body))
        })
    }
    fn swatches(
        &self,
        group: &'static str,
        colors: impl IntoIterator<Item = u32>,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut grid = div().flex().flex_wrap();
        for (i, hex) in colors.into_iter().enumerate() {
            grid = grid.child(self.swatch((group, i).into(), hex, cx));
        }
        grid
    }
    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let mut tabs = surface().flex().p(px(2.));
        for (i, name) in ["Picker", "Sliders", "Palette"].into_iter().enumerate() {
            tabs = tabs.child(
                button(("tab", i), name)
                    .flex_1()
                    .h(px(34.))
                    .gap_2()
                    .child(icon([Icon::Picker, Icon::Sliders, Icon::Palette][i]))
                    .child(name)
                    .aria_selected(self.tab == i)
                    .when(self.tab == i, |s| {
                        s.bg(rgb(0x382845)).border_color(rgb(0xa775da))
                    })
                    .when(self.tab != i, |s| {
                        s.border_color(transparent_black()).text_color(rgb(MUTED))
                    })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.tab = i;
                        cx.notify();
                    })),
            );
        }
        let mut body = div().flex().flex_col().gap_3().child(tabs);
        match self.tab {
            0 => {
                body = body.child(
                    self.group(
                        0,
                        "Presets",
                        self.swatches(
                            "preset",
                            PRESETS
                                .into_iter()
                                .chain(self.library.custom.iter().copied()),
                            cx,
                        ),
                        cx,
                    ),
                );
            }
            1 => {
                let mut sliders = surface().p_3().flex().flex_col().gap_2();
                for (i, name) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                    let n = channels(self.color.hex())[i];
                    sliders = sliders.child(
                        div()
                            .child(row().child(name).child(caption(n.to_string())))
                            .child(self.bar(
                                Track::Channel(i),
                                name,
                                n as f32 / 255.,
                                [0xea575e, 0x55c774, 0x6488ed][i],
                                cx,
                            )),
                    );
                }
                body = body.child(sliders);
            }
            _ => {
                let tones = (0..16).map(|i| {
                    Color {
                        s: if i < 8 {
                            self.color.s * (i + 1) as f32 / 8.
                        } else {
                            self.color.s
                        },
                        v: if i < 8 { 1. } else { (16 - i) as f32 / 9. },
                        ..self.color
                    }
                    .hex()
                });
                body = body.child(
                    surface()
                        .p_3()
                        .child(caption("Tints and shades"))
                        .child(self.swatches("tone", tones, cx)),
                );
            }
        }
        let recent = if self.library.recent.is_empty() {
            div()
                .p_1()
                .child(caption("Choose a color to start your history."))
        } else {
            self.swatches("recent", self.library.recent.iter().copied(), cx)
        };
        let mut favorites = div().flex().gap_2();
        for (i, slot) in self.library.favorites.iter().copied().enumerate() {
            favorites = favorites.child(
                button(
                    ("favorite", i),
                    slot.map(|hex| format!("Use favorite #{hex:06X}"))
                        .unwrap_or_else(|| "Save current color".into()),
                )
                .size(px(32.))
                .when_some(slot, |s, c| s.bg(rgb(c)))
                .when(slot.is_none(), |s| s.child("+"))
                .on_click(cx.listener(move |v, _, _, cx| {
                    if let Some(c) = v.library.favorites[i] {
                        v.select(c, cx);
                    } else {
                        v.library.favorites[i] = Some(v.color.hex());
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |v, _, _, cx| {
                        v.library.favorites[i] = None;
                        cx.notify();
                    }),
                ),
            );
        }
        body.child(self.group(1, "Recently used", recent, cx))
            .child(self.group(2, "Favorites", favorites, cx))
    }
    fn finish(&mut self, save: bool, w: &mut Window, cx: &mut Context<Self>) {
        if save {
            self.library.selected = self.color.hex();
            self.library.remember(self.color.hex());
            if let Err(e) = self.library.save() {
                self.message = Some(format!("Could not save colors: {e}"));
                cx.notify();
                return;
            }
        } else if self.filter != self.initial {
            self.filter = self.initial;
            self.apply(cx);
        }
        w.remove_window();
    }
}

impl Render for Panel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = w.viewport_size().width < px(830.);
        let scope = if self.legacy_tint() {
            "Tint preview · Sign out and back in for desktop tint"
        } else if self.bus.is_some() {
            "Desktop filter · GNOME"
        } else {
            "Preview only"
        };
        let controls = div()
            .flex()
            .flex_col()
            .gap_4()
            .min_w_0()
            .p_5()
            .when(!compact, |s| s.flex_1())
            .child(self.color_box(cx))
            .child(self.readouts(cx))
            .child(self.filters(cx));
        let footer = row()
            .flex_wrap()
            .justify_end()
            .child(
                button("cancel", "Cancel")
                    .px_4()
                    .h(px(36.))
                    .bg(rgb(0x302738))
                    .child("Cancel")
                    .on_click(cx.listener(|v, _, w, cx| v.finish(false, w, cx))),
            )
            .child(
                button("done", "Done")
                    .px_5()
                    .h(px(36.))
                    .bg(rgb(0x7840d5))
                    .child("Done")
                    .on_click(cx.listener(|v, _, w, cx| v.finish(true, w, cx))),
            );
        div()
            .id("panel")
            .track_focus(&self.root_focus)
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(BASE))
            .text_color(rgb(TEXT))
            .font_family("Ubuntu")
            .on_key_down(cx.listener(
                |v, e: &KeyDownEvent, w, cx| match e.keystroke.key.as_str() {
                    "tab" => {
                        if e.keystroke.modifiers.shift {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        }
                    }
                    "escape" if v.menu => {
                        v.menu = false;
                        cx.notify();
                    }
                    _ => {}
                },
            ))
            .on_mouse_move(cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                if let Some(t) = v.drag {
                    if e.pressed_button == Some(MouseButton::Left) {
                        v.pointer(t, e.position, cx);
                    } else {
                        v.drag = None;
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| {
                    if let Some(t) = v.drag.take()
                        && t != Track::Intensity
                    {
                        v.library.remember(v.color.hex());
                        cx.notify();
                    }
                }),
            )
            .child(
                row()
                    .h(px(50.))
                    .px_5()
                    .bg(rgb(CARD))
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(30.))
                                    .rounded_md()
                                    .bg(rgb(0x382940))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(Icon::Picker)),
                            )
                            .child("Choose Color"),
                    )
                    .child(caption(
                        self.message.clone().unwrap_or_else(|| scope.into()),
                    )),
            )
            .child(
                div()
                    .flex()
                    .when(compact, |s| s.flex_col())
                    .child(controls)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .gap_4()
                            .p_4()
                            .bg(rgb(0x1b1624))
                            .when(!compact, |s| {
                                s.w(relative(0.36)).border_l_1().border_color(rgb(LINE))
                            })
                            .child(self.sidebar(cx))
                            .child(footer),
                    ),
            )
    }
}

fn main() {
    application().run(|cx| {
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit()
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1060.), px(645.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(420.), px(360.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Chroma Panel".into()),
                    ..Default::default()
                }),
                app_id: Some("local.ChromaPanel".into()),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Panel::new(focus)
                })
            },
        )
        .expect("open Chroma Panel");
        cx.activate(true);
    });
}

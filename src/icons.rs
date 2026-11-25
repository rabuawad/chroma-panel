use gpui::{prelude::*, *};

#[derive(Clone, Copy)]
pub enum Icon {
    Picker,
    Sliders,
    Palette,
    Sun,
    Filters,
}
pub fn icon(kind: Icon) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let mut pen = PathBuilder::stroke(px(1.35));
            let mut line = |points: &[(f32, f32)]| {
                for (i, &(x, y)) in points.iter().enumerate() {
                    let p = b.origin + point(px(x), px(y));
                    if i == 0 {
                        pen.move_to(p);
                    } else {
                        pen.line_to(p);
                    }
                }
            };
            match kind {
                Icon::Picker => {
                    line(&[
                        (2., 16.),
                        (3., 12.),
                        (11., 4.),
                        (14., 7.),
                        (6., 15.),
                        (2., 16.),
                    ]);
                    line(&[(9., 3.), (15., 9.)]);
                    line(&[(12., 4.), (14., 2.), (16., 4.), (14., 6.)]);
                }
                Icon::Sliders => {
                    for (y, x) in [(4., 6.), (9., 12.), (14., 7.)] {
                        line(&[(1., y), (x - 2., y)]);
                        line(&[(x + 2., y), (17., y)]);
                        line(&[
                            (x - 2., y),
                            (x, y - 2.),
                            (x + 2., y),
                            (x, y + 2.),
                            (x - 2., y),
                        ]);
                    }
                }
                Icon::Palette => {
                    for (x, y) in [(2., 2.), (10., 2.), (2., 10.), (10., 10.)] {
                        line(&[(x, y), (x + 6., y), (x + 6., y + 6.), (x, y + 6.), (x, y)]);
                    }
                }
                Icon::Sun | Icon::Filters => {
                    let dots: &[(f32, f32, f32)] = match kind {
                        Icon::Sun => &[(9., 9., 4.)],
                        _ => &[(5., 5., 2.), (13., 5., 2.), (5., 13., 2.), (13., 13., 2.)],
                    };
                    for &(x, y, r) in dots {
                        let ring: Vec<_> = (0..=16)
                            .map(|i| {
                                let a = i as f32 * std::f32::consts::TAU / 16.;
                                (x + r * a.cos(), y + r * a.sin())
                            })
                            .collect();
                        line(&ring);
                    }
                    if matches!(kind, Icon::Sun) {
                        for i in 0..8 {
                            let a = i as f32 * std::f32::consts::FRAC_PI_4;
                            line(&[
                                (9. + 6. * a.cos(), 9. + 6. * a.sin()),
                                (9. + 8. * a.cos(), 9. + 8. * a.sin()),
                            ]);
                        }
                    }
                }
            }
            if let Ok(path) = pen.build() {
                w.paint_path(path, rgb(0xcbb8e1));
            }
        },
    )
    .size(px(18.))
    .flex_none()
}

use crate::dsp::{Biquad, BiquadKind, Coefficients};
use crate::{BandParams, EmberParams, FilterKind};
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};
use std::f32::consts::PI;
use std::sync::Arc;

const CREAM: egui::Color32 = egui::Color32::from_rgb(246, 237, 220);
const PAPER: egui::Color32 = egui::Color32::from_rgb(236, 220, 194);
const PAPER_HOVER: egui::Color32 = egui::Color32::from_rgb(229, 205, 172);
const INK: egui::Color32 = egui::Color32::from_rgb(44, 33, 27);
const MUTED: egui::Color32 = egui::Color32::from_rgb(117, 100, 88);
const RUST: egui::Color32 = egui::Color32::from_rgb(184, 93, 55);
const MOSS: egui::Color32 = egui::Color32::from_rgb(95, 111, 80);
const AMBER: egui::Color32 = egui::Color32::from_rgb(217, 145, 58);
const GRAPH_BG: egui::Color32 = egui::Color32::from_rgb(48, 42, 35);
const GRAPH_GRID: egui::Color32 = egui::Color32::from_rgb(78, 69, 59);
const GRAPH_TEXT: egui::Color32 = egui::Color32::from_rgb(194, 180, 160);
const RESPONSE_LIMIT_DB: f32 = 18.0;
const DISPLAY_SAMPLE_RATE: f32 = 48_000.0;

#[derive(Default)]
struct GuiState {
    active_band: Option<usize>,
}

pub fn create(params: Arc<EmberParams>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        GuiState::default(),
        |ctx, _state| {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = CREAM;
            visuals.window_fill = CREAM;
            visuals.override_text_color = Some(INK);
            visuals.widgets.inactive.bg_fill = PAPER;
            visuals.widgets.hovered.bg_fill = PAPER_HOVER;
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(218, 184, 143);
            visuals.selection.bg_fill = RUST;
            visuals.selection.stroke.color = CREAM;
            ctx.set_visuals(visuals);

            let mut style = (*ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, state| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(CREAM)
                        .inner_margin(egui::Margin::same(20)),
                )
                .show(ctx, |ui| {
                    draw_header(ui, &params, setter);
                    ui.add_space(14.0);
                    draw_response_graph(ui, &params, setter, state);
                    ui.add_space(14.0);
                    draw_band_grid(ui, &params, setter);
                    ui.add_space(12.0);
                    draw_footer(ui, &params, setter);
                });
        },
    )
}

fn draw_header(ui: &mut egui::Ui, params: &EmberParams, setter: &ParamSetter) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("EMBER EQ")
                    .size(30.0)
                    .strong()
                    .color(RUST),
            );
            ui.label(
                egui::RichText::new("Cove Labs  /  six-band parametric equalizer")
                    .size(13.0)
                    .color(MUTED),
            );
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            draw_bool_button(ui, &params.bypass, setter, "BYPASSED", "ACTIVE");
            ui.label(egui::RichText::new("Plugin").strong().color(INK));
        });
    });
}

fn draw_response_graph(
    ui: &mut egui::Ui,
    params: &EmberParams,
    setter: &ParamSetter,
    state: &mut GuiState,
) {
    let width = ui.available_width();
    let desired = egui::vec2(width, 265.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, egui::CornerRadius::same(18), GRAPH_BG);

    let inner = rect.shrink2(egui::vec2(18.0, 14.0));
    draw_graph_grid(&painter, inner);
    draw_response_curve(&painter, inner, params);

    let node_positions: Vec<egui::Pos2> = params
        .bands
        .iter()
        .map(|band| band_position(inner, band))
        .collect();

    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            state.active_band = nearest_node(pointer, &node_positions, 30.0);
        }
    }

    if response.dragged() {
        if let (Some(index), Some(pointer)) = (state.active_band, response.interact_pointer_pos()) {
            let band = &params.bands[index];
            let clamped_x = pointer.x.clamp(inner.left(), inner.right());
            let normalized_x = ((clamped_x - inner.left()) / inner.width()).clamp(0.0, 1.0);
            let frequency = normalized_to_frequency(normalized_x);

            setter.begin_set_parameter(&band.frequency);
            setter.set_parameter(&band.frequency, frequency);
            setter.end_set_parameter(&band.frequency);

            if !matches!(band.kind.value(), FilterKind::LowCut | FilterKind::HighCut) {
                let clamped_y = pointer.y.clamp(inner.top(), inner.bottom());
                let normalized_y = ((clamped_y - inner.top()) / inner.height()).clamp(0.0, 1.0);
                let gain = RESPONSE_LIMIT_DB - normalized_y * RESPONSE_LIMIT_DB * 2.0;

                setter.begin_set_parameter(&band.gain);
                setter.set_parameter(&band.gain, gain);
                setter.end_set_parameter(&band.gain);
            }
        }
    }

    if response.drag_stopped() {
        state.active_band = None;
    }

    let hover_position = ui.ctx().input(|input| input.pointer.hover_pos());
    let hovered_band = hover_position
        .filter(|pos| inner.contains(*pos))
        .and_then(|pos| nearest_node(pos, &node_positions, 22.0));

    for (index, position) in node_positions.iter().enumerate() {
        let band = &params.bands[index];
        let is_active = state.active_band == Some(index);
        let is_hovered = hovered_band == Some(index);
        let color = if !band.enabled.value() {
            MUTED
        } else if index % 2 == 0 {
            RUST
        } else {
            MOSS
        };

        let radius = if is_active || is_hovered { 11.0 } else { 9.0 };
        painter.circle_filled(*position, radius + 3.0, GRAPH_BG);
        painter.circle_filled(*position, radius, color);
        painter.text(
            *position,
            egui::Align2::CENTER_CENTER,
            (index + 1).to_string(),
            egui::FontId::proportional(11.0),
            CREAM,
        );
    }

    let info_band = state.active_band.or(hovered_band);
    if let Some(index) = info_band {
        let band = &params.bands[index];
        let text = format!(
            "Band {}  ·  {}  ·  {}  ·  Q {:.2}",
            index + 1,
            filter_kind_name(band.kind.value()),
            format_frequency(band.frequency.value()),
            band.q.value(),
        );

        painter.text(
            inner.left_top() + egui::vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(12.0),
            CREAM,
        );
    } else {
        painter.text(
            inner.left_top() + egui::vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            "drag a node to shape the sound",
            egui::FontId::proportional(12.0),
            GRAPH_TEXT,
        );
    }
}

fn draw_graph_grid(painter: &egui::Painter, rect: egui::Rect) {
    for db in [-18.0_f32, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0] {
        let y = gain_to_y(rect, db);
        let stroke = if db == 0.0 {
            egui::Stroke::new(
                1.5_f32,
                egui::Color32::from_rgba_unmultiplied(255, 244, 224, 75),
            )
        } else {
            egui::Stroke::new(1.0_f32, GRAPH_GRID)
        };
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );

        if db != -18.0 && db != 18.0 {
            painter.text(
                egui::pos2(rect.right() - 6.0, y - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{db:+.0}"),
                egui::FontId::proportional(10.0),
                GRAPH_TEXT,
            );
        }
    }

    let frequency_markers = [
        20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 20_000.0,
    ];

    for frequency in frequency_markers {
        let x = frequency_to_x(rect, frequency);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0_f32, GRAPH_GRID),
        );

        painter.text(
            egui::pos2(x, rect.bottom() - 4.0),
            egui::Align2::CENTER_BOTTOM,
            frequency_label(frequency),
            egui::FontId::proportional(10.0),
            GRAPH_TEXT,
        );
    }
}

fn draw_response_curve(painter: &egui::Painter, rect: egui::Rect, params: &EmberParams) {
    let point_count = rect.width().max(300.0).min(700.0) as usize;
    let mut points = Vec::with_capacity(point_count + 1);

    for index in 0..=point_count {
        let t = index as f32 / point_count as f32;
        let frequency = normalized_to_frequency(t);
        let mut total_db = 0.0;

        for band in &params.bands {
            if !band.enabled.value() {
                continue;
            }

            let coefficients = Biquad::coefficients(
                biquad_kind(band.kind.value()),
                DISPLAY_SAMPLE_RATE,
                band.frequency.value(),
                band.gain.value(),
                band.q.value(),
            );
            total_db += magnitude_db(coefficients, frequency, DISPLAY_SAMPLE_RATE);
        }

        let x = egui::lerp(rect.left()..=rect.right(), t);
        let y = gain_to_y(rect, total_db.clamp(-RESPONSE_LIMIT_DB, RESPONSE_LIMIT_DB));
        points.push(egui::pos2(x, y));
    }

    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.5_f32, AMBER),
    ));
}

fn draw_band_grid(ui: &mut egui::Ui, params: &EmberParams, setter: &ParamSetter) {
    egui::Grid::new("ember-band-grid")
        .num_columns(3)
        .spacing(egui::vec2(12.0, 12.0))
        .show(ui, |ui| {
            for index in 0..6 {
                draw_band_card(ui, index, &params.bands[index], setter);
                if index % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

fn draw_band_card(
    ui: &mut egui::Ui,
    index: usize,
    band: &BandParams,
    setter: &ParamSetter,
) {
    egui::Frame::default()
        .fill(PAPER)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(280.0);

            ui.horizontal(|ui| {
                let accent = if index % 2 == 0 { RUST } else { MOSS };
                ui.label(
                    egui::RichText::new(format!("BAND {}", index + 1))
                        .strong()
                        .color(accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    draw_bool_button(ui, &band.enabled, setter, "ON", "OFF");
                });
            });

            ui.add_space(4.0);

            egui::ComboBox::from_id_salt(format!("filter-kind-{index}"))
                .selected_text(filter_kind_name(band.kind.value()))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for kind in [
                        FilterKind::Bell,
                        FilterKind::LowShelf,
                        FilterKind::HighShelf,
                        FilterKind::LowCut,
                        FilterKind::HighCut,
                    ] {
                        if ui
                            .selectable_label(band.kind.value() == kind, filter_kind_name(kind))
                            .clicked()
                        {
                            setter.begin_set_parameter(&band.kind);
                            setter.set_parameter(&band.kind, kind);
                            setter.end_set_parameter(&band.kind);
                        }
                    }
                });

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Frequency").size(11.0).color(MUTED));
            ui.add(widgets::ParamSlider::for_param(&band.frequency, setter));

            if !matches!(band.kind.value(), FilterKind::LowCut | FilterKind::HighCut) {
                ui.label(egui::RichText::new("Gain").size(11.0).color(MUTED));
                ui.add(widgets::ParamSlider::for_param(&band.gain, setter));
            } else {
                ui.label(
                    egui::RichText::new("Gain is not used for cut filters")
                        .size(10.0)
                        .color(MUTED),
                );
            }

            ui.label(egui::RichText::new("Q").size(11.0).color(MUTED));
            ui.add(widgets::ParamSlider::for_param(&band.q, setter));
        });
}

fn draw_footer(ui: &mut egui::Ui, params: &EmberParams, setter: &ParamSetter) {
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Output").strong().color(INK));
        ui.add_sized(
            egui::vec2(220.0, 20.0),
            widgets::ParamSlider::for_param(&params.output_gain, setter),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("software made somewhere warm")
                    .italics()
                    .size(12.0)
                    .color(MUTED),
            );
        });
    });
}

fn draw_bool_button(
    ui: &mut egui::Ui,
    param: &BoolParam,
    setter: &ParamSetter,
    true_text: &str,
    false_text: &str,
) {
    let value = param.value();
    let label = if value { true_text } else { false_text };
    if ui.selectable_label(value, label).clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !value);
        setter.end_set_parameter(param);
    }
}

fn band_position(rect: egui::Rect, band: &BandParams) -> egui::Pos2 {
    let x = frequency_to_x(rect, band.frequency.value());
    let display_gain = if matches!(band.kind.value(), FilterKind::LowCut | FilterKind::HighCut) {
        0.0
    } else {
        band.gain.value()
    };
    egui::pos2(x, gain_to_y(rect, display_gain))
}

fn nearest_node(pointer: egui::Pos2, nodes: &[egui::Pos2], radius: f32) -> Option<usize> {
    let max_distance_squared = radius * radius;
    nodes
        .iter()
        .enumerate()
        .map(|(index, position)| (index, pointer.distance_sq(*position)))
        .filter(|(_, distance)| *distance <= max_distance_squared)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

fn frequency_to_x(rect: egui::Rect, frequency: f32) -> f32 {
    let normalized = ((frequency.clamp(20.0, 20_000.0) / 20.0).ln() / 1000.0_f32.ln())
        .clamp(0.0, 1.0);
    egui::lerp(rect.left()..=rect.right(), normalized)
}

fn normalized_to_frequency(normalized: f32) -> f32 {
    20.0 * 1000.0_f32.powf(normalized.clamp(0.0, 1.0))
}

fn gain_to_y(rect: egui::Rect, gain_db: f32) -> f32 {
    let normalized =
        ((RESPONSE_LIMIT_DB - gain_db) / (RESPONSE_LIMIT_DB * 2.0)).clamp(0.0, 1.0);
    egui::lerp(rect.top()..=rect.bottom(), normalized)
}

fn biquad_kind(kind: FilterKind) -> BiquadKind {
    match kind {
        FilterKind::Bell => BiquadKind::Peak,
        FilterKind::LowShelf => BiquadKind::LowShelf,
        FilterKind::HighShelf => BiquadKind::HighShelf,
        FilterKind::LowCut => BiquadKind::HighPass,
        FilterKind::HighCut => BiquadKind::LowPass,
    }
}

fn magnitude_db(coefficients: Coefficients, frequency: f32, sample_rate: f32) -> f32 {
    let omega = 2.0 * PI * frequency / sample_rate;
    let cos_1 = omega.cos();
    let sin_1 = omega.sin();
    let cos_2 = (2.0 * omega).cos();
    let sin_2 = (2.0 * omega).sin();

    let numerator_real =
        coefficients.b0 + coefficients.b1 * cos_1 + coefficients.b2 * cos_2;
    let numerator_imag = -(coefficients.b1 * sin_1 + coefficients.b2 * sin_2);
    let denominator_real = 1.0 + coefficients.a1 * cos_1 + coefficients.a2 * cos_2;
    let denominator_imag = -(coefficients.a1 * sin_1 + coefficients.a2 * sin_2);

    let numerator_power = numerator_real * numerator_real + numerator_imag * numerator_imag;
    let denominator_power =
        denominator_real * denominator_real + denominator_imag * denominator_imag;

    let magnitude = (numerator_power / denominator_power.max(1.0e-12)).sqrt();
    20.0 * magnitude.max(1.0e-8).log10()
}

fn filter_kind_name(kind: FilterKind) -> &'static str {
    match kind {
        FilterKind::Bell => "Bell",
        FilterKind::LowShelf => "Low Shelf",
        FilterKind::HighShelf => "High Shelf",
        FilterKind::LowCut => "Low Cut",
        FilterKind::HighCut => "High Cut",
    }
}

fn format_frequency(frequency: f32) -> String {
    if frequency >= 1000.0 {
        format!("{:.2} kHz", frequency / 1000.0)
    } else {
        format!("{frequency:.0} Hz")
    }
}

fn frequency_label(frequency: f32) -> String {
    if frequency >= 1000.0 {
        let value = frequency / 1000.0;
        if (value - value.round()).abs() < 0.01 {
            format!("{value:.0}k")
        } else {
            format!("{value:.1}k")
        }
    } else {
        format!("{frequency:.0}")
    }
}

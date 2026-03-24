//! Charts module for cost and credits history visualization
//!
//! Provides bar charts similar to the macOS SwiftUI Charts

#![allow(dead_code)]

use eframe::egui::{self, Color32, RichText, Rounding, Vec2};

/// Model cost breakdown for a single day
#[derive(Clone, Debug)]
pub struct ModelBreakdown {
    pub model_name: String,
    pub cost_usd: f64,
}

/// A single data point for the chart
#[derive(Clone, Debug)]
pub struct ChartPoint {
    pub date: String,      // "2025-01-15" format
    pub value: f64,        // Cost in USD or credits used
    pub tokens: Option<i64>, // Optional token count
    pub model_breakdowns: Option<Vec<ModelBreakdown>>, // Optional model-level breakdown
}

impl ChartPoint {
    /// Create a new chart point
    pub fn new(date: String, value: f64) -> Self {
        Self {
            date,
            value,
            tokens: None,
            model_breakdowns: None,
        }
    }

    /// Builder: add tokens
    pub fn with_tokens(mut self, tokens: i64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Builder: add model breakdowns
    pub fn with_model_breakdowns(mut self, breakdowns: Vec<ModelBreakdown>) -> Self {
        self.model_breakdowns = Some(breakdowns);
        self
    }
}

/// Cost history chart widget
pub struct CostHistoryChart {
    points: Vec<ChartPoint>,
    selected_index: Option<usize>,
    bar_color: Color32,
    total_cost: Option<f64>,
    animation_start: Option<std::time::Instant>,
    is_animated: bool,
}

impl CostHistoryChart {
    pub fn new(points: Vec<ChartPoint>, bar_color: Color32) -> Self {
        let total_cost = if points.is_empty() {
            None
        } else {
            Some(points.iter().map(|p| p.value).sum())
        };

        Self {
            points,
            selected_index: None,
            bar_color,
            total_cost,
            animation_start: None,
            is_animated: false,
        }
    }

    /// Start the entrance animation
    pub fn animate_entrance(&mut self) {
        self.animation_start = Some(std::time::Instant::now());
        self.is_animated = true;
    }

    /// Render the chart
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.points.is_empty() {
            ui.label(
                RichText::new("No cost history data.")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let max_value = self.points.iter().map(|p| p.value).fold(0.0f64, f64::max);
        let total_value: f64 = self.points.iter().map(|p| p.value).sum();

        // If all values are zero, show empty state instead of invisible chart
        if total_value == 0.0 {
            ui.label(
                RichText::new("No costs recorded yet")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let peak_index = self.points.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap())
            .map(|(i, _)| i);

        // Chart area
        let chart_height = 30.0;
        let available_width = ui.available_width();
        let bar_width = (available_width / self.points.len() as f32) * 0.8;
        let bar_spacing = (available_width / self.points.len() as f32) * 0.2;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, chart_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        // Animation timing constants
        const TOTAL_ANIMATION_MS: f32 = 600.0;
        const STAGGER_PER_BAR_MS: f32 = 20.0;

        // Check if animation is still in progress
        let animation_needs_repaint = if self.is_animated {
            if let Some(start) = self.animation_start {
                let elapsed = start.elapsed().as_millis() as f32;
                let total_duration = TOTAL_ANIMATION_MS + (self.points.len() as f32 * STAGGER_PER_BAR_MS);
                elapsed < total_duration
            } else {
                false
            }
        } else {
            false
        };

        // Draw bars
        for (i, point) in self.points.iter().enumerate() {
            // Calculate bar height with minimum visibility
            let base_bar_height = if max_value > 0.0 {
                let proportional = (point.value / max_value) as f32 * (chart_height - 10.0);
                // Minimum bar height of 3px if there's any value, so bars are always visible
                if point.value > 0.0 {
                    proportional.max(3.0)
                } else {
                    // Zero value gets a tiny 1px bar so the chart structure is visible
                    1.0
                }
            } else {
                1.0 // Fallback minimum
            };

            // Apply entrance animation with staggered timing
            let bar_height = if self.is_animated {
                if let Some(start) = self.animation_start {
                    let elapsed = start.elapsed().as_millis() as f32;
                    let bar_delay = i as f32 * STAGGER_PER_BAR_MS;
                    let bar_elapsed = (elapsed - bar_delay).max(0.0);
                    let progress = (bar_elapsed / TOTAL_ANIMATION_MS).min(1.0);
                    // Ease-out curve: 1.0 - (1.0 - progress)^3
                    let eased = 1.0 - (1.0 - progress).powi(3);
                    base_bar_height * eased
                } else {
                    base_bar_height
                }
            } else {
                base_bar_height
            };

            let x = rect.left() + (i as f32 * (bar_width + bar_spacing)) + bar_spacing / 2.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - bar_height),
                Vec2::new(bar_width, bar_height),
            );

            // Check hover
            let is_hovered = response.hover_pos().map_or(false, |pos| {
                pos.x >= x && pos.x <= x + bar_width
            });

            if is_hovered {
                self.selected_index = Some(i);
            }

            // Bar color - peak gets yellow cap
            let color = if Some(i) == peak_index && bar_height > 5.0 {
                // Draw main bar
                let main_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height + 5.0),
                    Vec2::new(bar_width, bar_height - 5.0),
                );
                painter.rect_filled(main_rect, Rounding::same(2.0), self.bar_color);

                // Draw yellow peak cap
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, 5.0),
                );
                painter.rect_filled(cap_rect, Rounding::same(2.0), Color32::from_rgb(255, 200, 50));
                continue;
            } else if is_hovered {
                self.bar_color.gamma_multiply(1.2)
            } else {
                self.bar_color
            };

            painter.rect_filled(bar_rect, Rounding::same(2.0), color);
        }

        // Request repaint if animation is in progress
        if animation_needs_repaint {
            ui.ctx().request_repaint();
        }

        // Hover selection highlight
        if let Some(idx) = self.selected_index {
            if idx < self.points.len() {
                let x = rect.left() + (idx as f32 * (bar_width + bar_spacing));
                let highlight_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top()),
                    Vec2::new(bar_width + bar_spacing, chart_height),
                );
                painter.rect_filled(highlight_rect, Rounding::ZERO, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
            }
        }

        // Reset selection if not hovering
        if !response.hovered() {
            self.selected_index = None;
        }

        // Compact: Only show detail on hover, no default text
        if let Some(idx) = self.selected_index {
            if let Some(point) = self.points.get(idx) {
                let date_display = format_date_display(&point.date);
                let cost_display = format!("${:.2}", point.value);

                let detail = if let Some(tokens) = point.tokens {
                    format!("{}: {} · {} tokens", date_display, cost_display, format_tokens(tokens))
                } else {
                    format!("{}: {}", date_display, cost_display)
                };

                ui.label(
                    RichText::new(detail)
                        .size(10.0)
                        .color(Color32::GRAY),
                );
            }
        }
        // Removed: "Hover a bar for details" and "Total (30d)" texts for compact layout
    }
}

/// Credits history chart widget
pub struct CreditsHistoryChart {
    points: Vec<ChartPoint>,
    selected_index: Option<usize>,
    total_credits: Option<f64>,
}

impl CreditsHistoryChart {
    pub fn new(points: Vec<ChartPoint>) -> Self {
        let total_credits = if points.is_empty() {
            None
        } else {
            Some(points.iter().map(|p| p.value).sum())
        };

        Self {
            points,
            selected_index: None,
            total_credits,
        }
    }

    /// Render the chart
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.points.is_empty() {
            ui.label(
                RichText::new("No credits history data.")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let bar_color = Color32::from_rgb(73, 163, 176); // Teal color for credits
        let max_value = self.points.iter().map(|p| p.value).fold(0.0f64, f64::max);
        let peak_index = self.points.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap())
            .map(|(i, _)| i);

        // Chart area
        let chart_height = 30.0;
        let available_width = ui.available_width();
        let bar_width = (available_width / self.points.len() as f32) * 0.8;
        let bar_spacing = (available_width / self.points.len() as f32) * 0.2;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, chart_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        // Draw bars
        for (i, point) in self.points.iter().enumerate() {
            let bar_height = if max_value > 0.0 {
                (point.value / max_value) as f32 * (chart_height - 10.0)
            } else {
                0.0
            };

            let x = rect.left() + (i as f32 * (bar_width + bar_spacing)) + bar_spacing / 2.0;

            // Check hover
            let is_hovered = response.hover_pos().map_or(false, |pos| {
                pos.x >= x && pos.x <= x + bar_width
            });

            if is_hovered {
                self.selected_index = Some(i);
            }

            // Bar color - peak gets yellow cap
            if Some(i) == peak_index && bar_height > 5.0 {
                // Draw main bar
                let main_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height + 5.0),
                    Vec2::new(bar_width, bar_height - 5.0),
                );
                painter.rect_filled(main_rect, Rounding::same(2.0), bar_color);

                // Draw yellow peak cap
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, 5.0),
                );
                painter.rect_filled(cap_rect, Rounding::same(2.0), Color32::from_rgb(255, 200, 50));
            } else {
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, bar_height),
                );
                let color = if is_hovered {
                    bar_color.gamma_multiply(1.2)
                } else {
                    bar_color
                };
                painter.rect_filled(bar_rect, Rounding::same(2.0), color);
            }
        }

        // Reset selection if not hovering
        if !response.hovered() {
            self.selected_index = None;
        }

        ui.add_space(8.0);

        // Detail text
        if let Some(idx) = self.selected_index {
            if let Some(point) = self.points.get(idx) {
                let date_display = format_date_display(&point.date);
                let detail = format!("{}: {:.2} credits", date_display, point.value);

                ui.label(
                    RichText::new(detail)
                        .size(11.0)
                        .color(Color32::GRAY),
                );
            }
        } else {
            ui.label(
                RichText::new("Hover a bar for details")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }

        // Total
        if let Some(total) = self.total_credits {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Total (30d): {:.2} credits", total))
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }
    }
}

/// Format date from "2025-01-15" to "Jan 15"
fn format_date_display(date_key: &str) -> String {
    let parts: Vec<&str> = date_key.split('-').collect();
    if parts.len() != 3 {
        return date_key.to_string();
    }

    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };

    let day = parts[2].trim_start_matches('0');
    format!("{} {}", month, day)
}

/// Format token count with K/M suffix
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Service usage for a single day (for stacked charts)
#[derive(Clone, Debug)]
pub struct ServiceUsage {
    pub service: String,
    pub credits_used: f64,
}

/// A single data point for usage breakdown chart
#[derive(Clone, Debug)]
pub struct UsageBreakdownPoint {
    pub day: String,           // "2025-01-15" format
    pub services: Vec<ServiceUsage>,
    pub total_credits_used: f64,
}

impl UsageBreakdownPoint {
    pub fn new(day: String, services: Vec<ServiceUsage>) -> Self {
        let total_credits_used = services.iter().map(|s| s.credits_used).sum();
        Self {
            day,
            services,
            total_credits_used,
        }
    }
}

/// Usage breakdown chart widget (stacked bar chart by service)
pub struct UsageBreakdownChart {
    points: Vec<UsageBreakdownPoint>,
    selected_index: Option<usize>,
    service_colors: Vec<(String, Color32)>,
}

impl UsageBreakdownChart {
    pub fn new(points: Vec<UsageBreakdownPoint>) -> Self {
        // Build service color mapping
        let service_colors = Self::build_service_colors(&points);

        Self {
            points,
            selected_index: None,
            service_colors,
        }
    }

    fn build_service_colors(points: &[UsageBreakdownPoint]) -> Vec<(String, Color32)> {
        // Collect all unique services and their total usage
        let mut service_totals: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for point in points {
            for service in &point.services {
                *service_totals.entry(service.service.clone()).or_insert(0.0) += service.credits_used;
            }
        }

        // Sort by total usage descending
        let mut sorted: Vec<_> = service_totals.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Assign colors
        sorted.into_iter().map(|(service, _)| {
            let color = color_for_service(&service);
            (service, color)
        }).collect()
    }

    fn get_service_color(&self, service: &str) -> Color32 {
        self.service_colors.iter()
            .find(|(s, _)| s == service)
            .map(|(_, c)| *c)
            .unwrap_or(Color32::GRAY)
    }

    /// Render the chart
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.points.is_empty() {
            ui.label(
                RichText::new("No usage breakdown data.")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let max_value = self.points.iter()
            .map(|p| p.total_credits_used)
            .fold(0.0f64, f64::max);
        let peak_index = self.points.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.total_credits_used.partial_cmp(&b.total_credits_used).unwrap())
            .map(|(i, _)| i);

        // Chart area
        let chart_height = 80.0;
        let available_width = ui.available_width();
        let bar_width = (available_width / self.points.len() as f32) * 0.8;
        let bar_spacing = (available_width / self.points.len() as f32) * 0.2;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, chart_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        // Draw stacked bars
        for (i, point) in self.points.iter().enumerate() {
            let total_bar_height = if max_value > 0.0 {
                (point.total_credits_used / max_value) as f32 * (chart_height - 15.0)
            } else {
                0.0
            };

            let x = rect.left() + (i as f32 * (bar_width + bar_spacing)) + bar_spacing / 2.0;

            // Check hover
            let is_hovered = response.hover_pos().map_or(false, |pos| {
                pos.x >= x && pos.x <= x + bar_width
            });

            if is_hovered {
                self.selected_index = Some(i);
            }

            // Draw stacked segments from bottom to top
            let mut current_y = rect.bottom();
            for service in &point.services {
                if service.credits_used <= 0.0 {
                    continue;
                }

                let segment_height = if max_value > 0.0 {
                    (service.credits_used / max_value) as f32 * (chart_height - 15.0)
                } else {
                    0.0
                };

                let segment_rect = egui::Rect::from_min_size(
                    egui::pos2(x, current_y - segment_height),
                    Vec2::new(bar_width, segment_height),
                );

                let mut color = self.get_service_color(&service.service);
                if is_hovered {
                    color = color.gamma_multiply(1.2);
                }

                painter.rect_filled(segment_rect, Rounding::same(1.0), color);
                current_y -= segment_height;
            }

            // Draw yellow peak cap on highest day
            if Some(i) == peak_index && total_bar_height > 5.0 {
                let cap_height = 4.0;
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - total_bar_height - cap_height),
                    Vec2::new(bar_width, cap_height),
                );
                painter.rect_filled(cap_rect, Rounding::same(2.0), Color32::from_rgb(255, 200, 50));
            }
        }

        // Reset selection if not hovering
        if !response.hovered() {
            self.selected_index = None;
        }

        ui.add_space(6.0);

        // Detail text on hover
        if let Some(idx) = self.selected_index {
            if let Some(point) = self.points.get(idx) {
                let date_display = format_date_display(&point.day);
                let total_display = format!("{:.1}", point.total_credits_used);

                // Show top services
                let top_services: String = point.services.iter()
                    .filter(|s| s.credits_used > 0.0)
                    .take(3)
                    .map(|s| format!("{} {:.1}", s.service, s.credits_used))
                    .collect::<Vec<_>>()
                    .join(" · ");

                ui.label(
                    RichText::new(format!("{}: {} credits", date_display, total_display))
                        .size(10.0)
                        .color(Color32::GRAY),
                );
                if !top_services.is_empty() {
                    ui.label(
                        RichText::new(top_services)
                            .size(10.0)
                            .color(Color32::GRAY),
                    );
                }
            }
        }

        // Legend
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for (service, color) in &self.service_colors {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 7.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.5, *color);
                ui.label(RichText::new(service).size(10.0).color(Color32::GRAY));
            }
        });
    }
}

/// Get color for a service name
fn color_for_service(service: &str) -> Color32 {
    let lower = service.to_lowercase();

    if lower == "cli" {
        return Color32::from_rgb(66, 140, 245); // Blue
    }
    if lower.contains("github") && lower.contains("review") {
        return Color32::from_rgb(240, 135, 46); // Orange
    }
    if lower.contains("api") {
        return Color32::from_rgb(117, 191, 92); // Green
    }

    // Palette for other services
    let palette = [
        Color32::from_rgb(204, 115, 235), // Purple
        Color32::from_rgb(66, 199, 219),  // Cyan
        Color32::from_rgb(240, 188, 66),  // Yellow
        Color32::from_rgb(235, 87, 87),   // Red
        Color32::from_rgb(156, 156, 156), // Gray
    ];

    let idx = service.bytes().map(|b| b as usize).sum::<usize>() % palette.len();
    palette[idx]
}

/// Format top models text for chart tooltip
fn format_top_models(breakdowns: &[ModelBreakdown]) -> String {
    if breakdowns.is_empty() {
        return String::new();
    }

    // Sort by cost descending and take top 3
    let mut sorted: Vec<_> = breakdowns.iter()
        .filter(|b| b.cost_usd > 0.0)
        .collect();
    sorted.sort_by(|a, b| b.cost_usd.partial_cmp(&a.cost_usd).unwrap_or(std::cmp::Ordering::Equal));

    let top: Vec<String> = sorted.iter()
        .take(3)
        .map(|b| format!("{} ${:.2}", format_model_name(&b.model_name), b.cost_usd))
        .collect();

    if top.is_empty() {
        return String::new();
    }

    format!("Top: {}", top.join(" · "))
}

/// Format model name for display (shorten long names)
fn format_model_name(name: &str) -> String {
    // Common model name mappings
    let name_lower = name.to_lowercase();

    if name_lower.contains("claude-3.5-sonnet") || name_lower.contains("claude-3-5-sonnet") {
        return "Sonnet 3.5".to_string();
    }
    if name_lower.contains("claude-3-opus") || name_lower.contains("claude-3.opus") {
        return "Opus 3".to_string();
    }
    if name_lower.contains("claude-opus-4") || name_lower.contains("claude-4-opus") {
        return "Opus 4".to_string();
    }
    if name_lower.contains("claude-sonnet-4") || name_lower.contains("claude-4-sonnet") {
        return "Sonnet 4".to_string();
    }
    if name_lower.contains("claude-3-haiku") {
        return "Haiku 3".to_string();
    }
    if name_lower.contains("gpt-4o") {
        return "GPT-4o".to_string();
    }
    if name_lower.contains("gpt-4-turbo") {
        return "GPT-4T".to_string();
    }
    if name_lower.contains("gpt-4") {
        return "GPT-4".to_string();
    }
    if name_lower.contains("gemini-1.5-pro") || name_lower.contains("gemini-1-5-pro") {
        return "Gemini Pro".to_string();
    }
    if name_lower.contains("gemini-1.5-flash") || name_lower.contains("gemini-1-5-flash") {
        return "Gemini Flash".to_string();
    }
    if name_lower.contains("gemini-2") {
        return "Gemini 2".to_string();
    }

    // Truncate long names
    if name.len() > 15 {
        format!("{}...", &name[..12])
    } else {
        name.to_string()
    }
}

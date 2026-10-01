use super::gutter::Gutter;
use super::theme::Theme;
use crate::diagnostics::catalog::DiagnosticTemplate;
use crate::diagnostics::ingress::DynamicPayload;

fn get_terminal_width() -> usize {
    #[cfg(unix)]
    {
        #[repr(C)]
        struct Winsize {
            ws_row: u16,
            ws_col: u16,
            ws_xpixel: u16,
            ws_ypixel: u16,
        }

        unsafe extern "C" {
            fn ioctl(fd: i32, request: u64, ...) -> i32;
        }

        let mut ws = Winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        for &fd in &[2, 1] {
            unsafe {
                if ioctl(fd, 0x5413, &mut ws) == 0 && ws.ws_col > 0 {
                    return (ws.ws_col as usize).clamp(60, 120);
                }
            }
        }
    }

    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(c) = cols.parse::<usize>() {
            return c.clamp(60, 120);
        }
    }

    100
}

fn wrap_footnote(indent: &str, prefix_colored: &str, text: &str, max_width: usize) -> String {
    let mut out = String::with_capacity(text.len() + 64);

    let prefix_plain_len = 8;
    let hanging_indent = " ".repeat(indent.len() + prefix_plain_len);

    let available_width = max_width
        .saturating_sub(indent.len() + prefix_plain_len)
        .max(20);

    let words = text.split_whitespace();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in words {
        let word_len = word.chars().count();
        if current_line.is_empty() {
            current_line.push_str(word);
            current_len = word_len;
        } else if current_len + 1 + word_len <= available_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_len += 1 + word_len;
        } else {
            lines.push(current_line);
            current_line = word.to_string();
            current_len = word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("{indent}{prefix_colored} {line}\n"));
        } else {
            out.push_str(&format!("{hanging_indent}{line}\n"));
        }
    }

    out
}

pub fn render_footnotes(
    template: &DiagnosticTemplate,
    payload: Option<&DynamicPayload>,
    gutter: &Gutter,
    theme: &Theme,
) -> String {
    let mut out = String::with_capacity(256);
    let indent = gutter.footnote_indent();
    let max_width = get_terminal_width();

    if let Some(note_str) = template.note {
        for line in note_str.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    trimmed,
                    max_width,
                ));
            }
        }
    }

    if let Some(data) = payload {
        match data {
            DynamicPayload::VulkanDriverNotFound { error_details } => {
                let msg = format!("driver or runtime loader failure: {error_details}");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg,
                    max_width,
                ));
            }

            DynamicPayload::MissingHardwareFeatures {
                device_name,
                missing_features,
            } => {
                let msg1 = format!("target device: '{device_name}'");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));

                let features_list = missing_features.join(", ");
                let msg2 = format!(
                    "the following mandatory compute features are unsupported: {features_list}"
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::IntegratedGpuAlert {
                selected_gpu,
                available_dgpu,
            } => {
                let msg1 = format!("active compute device: '{selected_gpu}' (integrated)");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));

                if let Some(dgpu) = available_dgpu {
                    let msg2 = format!("discrete high-performance GPU detected: '{dgpu}'");
                    out.push_str(&wrap_footnote(
                        &indent,
                        theme.note_prefix(),
                        &msg2,
                        max_width,
                    ));
                }
            }

            DynamicPayload::ParamArenaClamped {
                requested_bytes,
                max_supported_bytes,
            } => {
                let req_mb = *requested_bytes as f64 / (1024.0 * 1024.0);
                let max_mb = *max_supported_bytes as f64 / (1024.0 * 1024.0);

                let msg1 = format!("configured arena size: {:.2} MB", req_mb);
                let msg2 = format!(
                    "device maximum allocation capacity: {:.2} MB (allocation clamped to prevent VRAM allocation fault)",
                    max_mb
                );

                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::ArityMismatch {
                nam_name,
                expected_count,
                provided_count,
            } => {
                let msg = format!(
                    "`#[nam] fn {nam_name}` requires {expected_count} arguments, but {provided_count} were supplied"
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg,
                    max_width,
                ));
            }

            DynamicPayload::MutabilityMismatch {
                param_name,
                expected_mut,
                provided_type,
            } => {
                let exp_str = if *expected_mut {
                    "an exclusive mutable reference (`&mut`)"
                } else {
                    "a shared read-only reference (`&`)"
                };
                let msg1 = format!("parameter `{param_name}` expects {exp_str}");
                let msg2 =
                    format!("the caller passed `{provided_type}`, which is an immutable borrow");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::SemanticKindMismatch {
                param_name,
                expected_kind_str,
                provided_kind_str,
            } => {
                let msg1 = format!("parameter `{param_name}` expects {expected_kind_str}");
                let msg2 = format!("the caller passed a {provided_kind_str}");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::TypeMismatch {
                param_name,
                expected_type,
                provided_type,
            } => {
                let msg1 =
                    format!("parameter `{param_name}` requires elements of type `{expected_type}`");
                let msg2 =
                    format!("the provided container contains elements of type `{provided_type}`");
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::OverlappingSlices {
                first_arg,
                first_range,
                second_arg,
                second_range,
                intersection,
                element_type,
            } => {
                let overlap_count = intersection.end.saturating_sub(intersection.start);
                let msg1 = format!(
                    "argument {} covers elements [{}..{}]",
                    first_arg + 1,
                    first_range.start,
                    first_range.end
                );
                let msg2 = format!(
                    "argument {} covers elements [{}..{}]",
                    second_arg + 1,
                    second_range.start,
                    second_range.end
                );
                let msg3 = format!(
                    "spatial collision occurs on {overlap_count} overlapping elements [{}..{}] of the parent GpuVec<{element_type}>",
                    intersection.start, intersection.end
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg3,
                    max_width,
                ));
            }

            DynamicPayload::SpaceDomainOverflow {
                arg_index,
                type_name,
                provided_elements,
                required_elements,
                space_dimensions,
            } => {
                let deficit = required_elements.saturating_sub(*provided_elements);
                let msg1 = format!(
                    "argument {} (`GpuVec<{type_name}>`) contains only {provided_elements} elements",
                    arg_index + 1
                );
                let msg2 = format!(
                    "space execution (dimensions: {}x{}x{}) requires at least {required_elements} elements (deficit of {deficit} elements)",
                    space_dimensions.0, space_dimensions.1, space_dimensions.2
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::UnrestrictedSliceMut {
                arg_index,
                type_name,
                element_range,
                total_cells,
            } => {
                let msg1 = format!(
                    "argument {} is an unrestricted mutable slice `SliceMut<{type_name}>` covering elements [{}..{}]",
                    arg_index + 1,
                    element_range.start,
                    element_range.end
                );
                let msg2 = format!(
                    "safe dispatch executing over {total_cells} parallel threads cannot guarantee race-free indexing"
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::TemporalPresentationHazard { arg_index, root_id } => {
                let msg = format!(
                    "argument {} (parent GpuVec #{root_id}) was already queued for screen presentation via `.present()` earlier in this flow",
                    arg_index + 1
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg,
                    max_width,
                ));
            }

            DynamicPayload::StackOverflow {
                required_bytes,
                available_bytes,
                total_bytes,
                stack_per_cell,
                total_cells,
            } => {
                let req_mb = *required_bytes as f64 / (1024.0 * 1024.0);
                let req_gb = *required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let avail_mb = *available_bytes as f64 / (1024.0 * 1024.0);
                let total_gb = *total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

                let msg1 = format!(
                    "requested stack: {:.2} MB ({:.2} GB) ({} bytes/cell across {} cells)",
                    req_mb, req_gb, stack_per_cell, total_cells
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));

                let msg2 = format!(
                    "available VRAM: {:.2} MB (out of {:.2} GB total)",
                    avail_mb, total_gb
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::DimensionMismatch {
                expected_width,
                expected_height,
                actual_elements,
            } => {
                let expected_total = (expected_width * expected_height) as usize;
                let msg1 = format!(
                    "target window extent is {}x{} (expected {} elements)",
                    expected_width, expected_height, expected_total
                );
                let msg2 = format!("provided buffer only contains {} elements", actual_elements);
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg1,
                    max_width,
                ));
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg2,
                    max_width,
                ));
            }

            DynamicPayload::PhaseViolation { operation } => {
                let msg = format!(
                    "`{operation}()` was invoked on the CPU while recording GPU commands inside `enki.frame`"
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg,
                    max_width,
                ));
            }

            DynamicPayload::TimestampQueriesExceeded {
                requested,
                max_capacity,
            } => {
                let msg = format!(
                    "attempted to record timestamp query #{requested}, but maximum capacity is limited to {max_capacity} queries per frame"
                );
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    &msg,
                    max_width,
                ));
            }

            DynamicPayload::InternalCompilerError { token } => {
                let note_msg = "the compiler caught an internal failure and sealed the state into an encrypted token";
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.note_prefix(),
                    note_msg,
                    max_width,
                ));

                out.push('\n');
                for line in token.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        out.push_str(&format!("{indent}  \x1b[2;37m{trimmed}\x1b[0m\n"));
                    }
                }
                out.push('\n');
            }
        }
    }

    if let Some(help_str) = template.help {
        for line in help_str.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push_str(&wrap_footnote(
                    &indent,
                    theme.help_prefix(),
                    trimmed,
                    max_width,
                ));
            }
        }
    }

    out
}

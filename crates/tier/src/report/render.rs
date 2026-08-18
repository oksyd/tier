use std::fmt::Write as _;

use serde_json::Value;

use super::model::DoctorReport;

pub(super) fn render_doctor(doctor: &DoctorReport) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "Config Doctor");
    let _ = writeln!(&mut output, "Format: v{}", doctor.format_version);
    let _ = writeln!(&mut output, "Sources: {}", doctor.summary.source_count);
    for source in &doctor.sources {
        let _ = writeln!(&mut output, "- {source}");
    }

    let _ = writeln!(
        &mut output,
        "Validations: {}",
        doctor.summary.validation_count
    );
    for validation in &doctor.validations {
        let _ = writeln!(&mut output, "- {validation}");
    }

    let _ = writeln!(&mut output, "Traces: {}", doctor.summary.trace_count);
    let _ = writeln!(&mut output, "Secrets: {}", doctor.summary.secret_path_count);
    let _ = writeln!(
        &mut output,
        "Migrations: {}",
        doctor.summary.migration_count
    );
    for migration in &doctor.migrations {
        let _ = writeln!(&mut output, "- {migration}");
    }

    if doctor.warnings.is_empty() {
        let _ = writeln!(&mut output, "Warnings: 0");
    } else {
        let _ = writeln!(&mut output, "Warnings: {}", doctor.summary.warning_count);
        for warning in &doctor.warnings {
            let _ = writeln!(&mut output, "- {warning}");
        }
    }

    output
}

pub(super) fn render_value(value: &Value) -> String {
    match value {
        Value::String(inner) => inner.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "<unrenderable>".to_owned()),
    }
}

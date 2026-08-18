use serde_json::Value;

use crate::export::{json_pretty, json_value};

use super::{
    AuditReport, ConfigReport, DoctorReport, REPORT_FORMAT_VERSION, TraceAudit,
    render::render_doctor,
};

impl ConfigReport {
    /// Returns the final redacted configuration rendered as pretty JSON.
    #[must_use]
    pub fn redacted_pretty_json(&self) -> String {
        json_pretty(
            &self.redacted_value(),
            "{\"error\":\"failed to render report\"}",
        )
    }

    /// Builds a machine-readable operational summary of the loaded configuration.
    #[must_use]
    pub fn doctor_report(&self) -> DoctorReport {
        DoctorReport {
            format_version: REPORT_FORMAT_VERSION,
            summary: self.summary(),
            sources: self.applied_sources.clone(),
            validations: self.validations.clone(),
            warnings: self.warnings.clone(),
            migrations: self.migrations.clone(),
            redacted_final: self.redacted_value(),
        }
    }

    /// Builds a machine-readable audit payload including all path traces.
    #[must_use]
    pub fn audit_report(&self) -> AuditReport {
        let traces = self
            .traces
            .keys()
            .filter_map(|path| {
                self.explain(path).map(|explanation| {
                    (
                        path.clone(),
                        TraceAudit {
                            last_source: explanation.steps.last().map(|step| step.source.clone()),
                            step_count: explanation.steps.len(),
                            explanation,
                        },
                    )
                })
            })
            .collect();

        AuditReport {
            format_version: REPORT_FORMAT_VERSION,
            summary: self.summary(),
            doctor: self.doctor_report(),
            traces,
        }
    }

    /// Renders a human-readable operational summary of the loaded configuration.
    #[must_use]
    pub fn doctor(&self) -> String {
        render_doctor(&self.doctor_report())
    }

    /// Renders a machine-readable operational summary of the loaded configuration.
    #[must_use]
    pub fn doctor_json(&self) -> Value {
        json_value(&self.doctor_report(), Value::Object(Default::default()))
    }

    /// Renders the machine-readable doctor output as pretty JSON.
    #[must_use]
    pub fn doctor_json_pretty(&self) -> String {
        json_pretty(
            &self.doctor_json(),
            "{\"error\":\"failed to render doctor report\"}",
        )
    }

    /// Renders a machine-readable audit payload including path traces.
    #[must_use]
    pub fn audit_json(&self) -> Value {
        json_value(&self.audit_report(), Value::Object(Default::default()))
    }

    /// Renders the machine-readable audit output as pretty JSON.
    #[must_use]
    pub fn audit_json_pretty(&self) -> String {
        json_pretty(
            &self.audit_json(),
            "{\"error\":\"failed to render audit report\"}",
        )
    }
}

//! Thin process protocol over the Scheme AOT scheduler.

use crate::{
    DriverPhase, DriverResource, DriverStatus, DriverTransition, driver_request, driver_transition,
};

fn phase(value: &str) -> Option<DriverPhase> {
    match value {
        "await-proposal" => Some(DriverPhase::AwaitProposal),
        "await-closure" => Some(DriverPhase::AwaitClosure),
        "complete" => Some(DriverPhase::Complete),
        _ => None,
    }
}

fn resource(value: &str) -> Option<DriverResource> {
    match value {
        "model-proposal" => Some(DriverResource::ModelProposal),
        "mrr-closure" => Some(DriverResource::MrrClosure),
        _ => None,
    }
}

fn status(value: &str) -> Option<DriverStatus> {
    match value {
        "candidate" => Some(DriverStatus::Candidate),
        "admitted" => Some(DriverStatus::Admitted),
        "rejected" => Some(DriverStatus::Rejected),
        _ => None,
    }
}

fn phase_name(value: DriverPhase) -> &'static str {
    match value {
        DriverPhase::AwaitProposal => "await-proposal",
        DriverPhase::AwaitClosure => "await-closure",
        DriverPhase::Complete => "complete",
    }
}

fn resource_name(value: DriverResource) -> &'static str {
    match value {
        DriverResource::ModelProposal => "model-proposal",
        DriverResource::MrrClosure => "mrr-closure",
    }
}

/// Execute one fixed-width scheduler command without interpreting payload data.
pub fn run_driver_cli(arguments: &[String]) -> Result<&'static str, String> {
    match arguments {
        [command, phase_value] if command == "request" => {
            let phase = phase(phase_value).ok_or_else(|| "invalid phase".to_owned())?;
            driver_request(phase)
                .map_err(|error| error.to_string())?
                .map(resource_name)
                .ok_or_else(|| "complete".to_owned())
        }
        [
            command,
            phase_value,
            resource_value,
            status_value,
            cycle,
            max_cycles,
        ] if command == "transition" => {
            let transition = DriverTransition {
                phase: phase(phase_value).ok_or_else(|| "invalid phase".to_owned())?,
                resource: resource(resource_value).ok_or_else(|| "invalid resource".to_owned())?,
                status: status(status_value).ok_or_else(|| "invalid status".to_owned())?,
                cycle: cycle.parse().map_err(|_| "invalid cycle".to_owned())?,
                max_cycles: max_cycles
                    .parse()
                    .map_err(|_| "invalid max cycles".to_owned())?,
            };
            driver_transition(transition)
                .map(phase_name)
                .map_err(|error| error.to_string())
        }
        _ => Err(
            "usage: mrr-scheme-driver request PHASE | transition PHASE RESOURCE STATUS CYCLE MAX"
                .to_owned(),
        ),
    }
}

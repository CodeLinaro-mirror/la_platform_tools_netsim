// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use cell_actor::{CellAction, CellClient};
use grpcio::{RpcContext, UnarySink};
use netsim_model::{ChipClient, MODEM_STATE_RINGING};
use netsim_proto::{
    cell::{Cell, ExecuteCellRequest, GetCellRequest, ListCellsRequest, ListCellsResponse},
    cell_grpc::CellService,
};
use protobuf::well_known_types::empty::Empty;
use tracing::error;

#[derive(Clone)]
pub struct CellServiceImpl {
    client: CellClient,
}

impl CellServiceImpl {
    pub fn new(client: CellClient) -> Self {
        Self { client }
    }
}

fn map_proto_reg_status(
    proto_status: netsim_proto::cell::RegistrationStatus,
) -> netsim_model::RegistrationStatus {
    match proto_status {
        netsim_proto::cell::RegistrationStatus::NOT_REGISTERED => {
            netsim_model::RegistrationStatus::NotRegistered
        }
        netsim_proto::cell::RegistrationStatus::REGISTERED_HOME => {
            netsim_model::RegistrationStatus::RegisteredHome
        }
        netsim_proto::cell::RegistrationStatus::SEARCHING => {
            netsim_model::RegistrationStatus::Searching
        }
        netsim_proto::cell::RegistrationStatus::DENIED => netsim_model::RegistrationStatus::Denied,
        netsim_proto::cell::RegistrationStatus::ROAMING => {
            netsim_model::RegistrationStatus::Roaming
        }
        _ => netsim_model::RegistrationStatus::Unknown,
    }
}

impl CellService for CellServiceImpl {
    fn get(&mut self, ctx: RpcContext, req: GetCellRequest, sink: UnarySink<Cell>) {
        let client = self.client.clone();
        ctx.spawn(async move {
            let chip_id = netsim_model::ChipId(req.id);
            match client.read(chip_id).await {
                Ok(chip) => {
                    let mut cell = Cell::new();
                    cell.id = chip.id;
                    if let Some(netsim_model::ChipVariant::Cell(cell_info)) = chip.variant {
                        cell.state = cell_info.state;
                        cell.ringing = cell.state == MODEM_STATE_RINGING;
                    }
                    sink.success(cell);
                }
                Err(e) => {
                    error!("Get cell failed: {}", e);
                    let status = grpcio::RpcStatus::with_message(
                        grpcio::RpcStatusCode::NOT_FOUND,
                        format!("Failed to get cell: {}", e),
                    );
                    sink.fail(status);
                }
            }
        });
    }

    fn list(
        &mut self,
        ctx: RpcContext,
        _req: ListCellsRequest,
        sink: UnarySink<ListCellsResponse>,
    ) {
        let client = self.client.clone();
        ctx.spawn(async move {
            match client.list().await {
                Ok(chips) => {
                    let mut response = ListCellsResponse::new();
                    for chip in chips {
                        let mut cell = Cell::new();
                        cell.id = chip.id;
                        if let Some(netsim_model::ChipVariant::Cell(cell_info)) = chip.variant {
                            cell.state = cell_info.state;
                            cell.ringing = cell.state == MODEM_STATE_RINGING;
                            response.cells.push(cell);
                        }
                    }
                    sink.success(response);
                }
                Err(e) => {
                    error!("List cells failed: {}", e);
                    let status = grpcio::RpcStatus::with_message(
                        grpcio::RpcStatusCode::INTERNAL,
                        format!("List cells failed: {}", e),
                    );
                    sink.fail(status);
                }
            }
        });
    }

    fn execute(&mut self, ctx: RpcContext, req: ExecuteCellRequest, sink: UnarySink<Empty>) {
        let client = self.client.clone();
        ctx.spawn(async move {
            let chip_id = netsim_model::ChipId(req.id);

            let action = match req.action {
                Some(netsim_proto::cell::execute_cell_request::Action::IncomingCall(call_info)) => {
                    CellAction::IncomingCall { number: call_info.number }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::UpdateCall(_)) => {
                    CellAction::UpdateCall
                }
                Some(netsim_proto::cell::execute_cell_request::Action::EndCall(_)) => {
                    CellAction::EndCall
                }
                Some(netsim_proto::cell::execute_cell_request::Action::ReceiveSms(s)) => {
                    CellAction::ReceiveSms { sender: s.sender, text: s.text }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::SetSignalStrength(s)) => {
                    CellAction::SetSignalStrength { rssi: s.rssi, ber: s.ber }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::SetVoiceRegistration(r)) => {
                    let status = map_proto_reg_status(r.status.enum_value_or_default());
                    CellAction::SetVoiceRegistration { status }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::SetDataRegistration(r)) => {
                    let status = map_proto_reg_status(r.status.enum_value_or_default());
                    CellAction::SetDataRegistration { status }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::RemoteAnswer(_)) => {
                    CellAction::RemoteAnswer
                }
                Some(netsim_proto::cell::execute_cell_request::Action::RemoteHold(h)) => {
                    CellAction::RemoteHold { on_hold: h.on_hold }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::SetSimStatus(s)) => {
                    let present = match s.state.enum_value_or_default() {
                        netsim_proto::cell::set_sim_status::SimState::ABSENT => false,
                        netsim_proto::cell::set_sim_status::SimState::PRESENT => true,
                    };
                    CellAction::SetSimStatus { present }
                }
                Some(netsim_proto::cell::execute_cell_request::Action::SetNetworkTechnology(t)) => {
                    let tech = match t.tech.enum_value_or_default() {
                        netsim_proto::cell::set_network_technology::RadioTechnology::GSM => {
                            netsim_model::RadioTechnology::Gsm
                        }
                        netsim_proto::cell::set_network_technology::RadioTechnology::LTE => {
                            netsim_model::RadioTechnology::Lte
                        }
                        netsim_proto::cell::set_network_technology::RadioTechnology::NR => {
                            netsim_model::RadioTechnology::Nr
                        }
                        _ => netsim_model::RadioTechnology::Unknown,
                    };
                    CellAction::SetNetworkTechnology { tech }
                }
                Some(_) => {
                    sink.fail(grpcio::RpcStatus::with_message(
                        grpcio::RpcStatusCode::UNIMPLEMENTED,
                        "unsupported action".into(),
                    ));
                    return;
                }
                None => {
                    sink.fail(grpcio::RpcStatus::with_message(
                        grpcio::RpcStatusCode::INVALID_ARGUMENT,
                        "missing action".into(),
                    ));
                    return;
                }
            };

            match client.perform_action(chip_id, action).await {
                Ok(_) => {
                    sink.success(Empty::new());
                }
                Err(e) => {
                    error!("Execute cell action failed: {}", e);
                    sink.fail(grpcio::RpcStatus::with_message(
                        grpcio::RpcStatusCode::INTERNAL,
                        format!("Failed to execute action: {}", e),
                    ));
                }
            }
        });
    }
}

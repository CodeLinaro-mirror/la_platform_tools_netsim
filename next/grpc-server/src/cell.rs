// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use grpcio::{RpcContext, UnarySink};
use netsim_proto::{
    cell::{Cell, ExecuteCellRequest, GetCellRequest, ListCellsRequest, ListCellsResponse},
    cell_grpc::CellService,
};
use protobuf::well_known_types::empty::Empty;

#[derive(Clone)]
pub struct CellServiceImpl;

impl CellServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl CellService for CellServiceImpl {
    fn get(&mut self, ctx: RpcContext, _req: GetCellRequest, sink: UnarySink<Cell>) {
        ctx.spawn(async move {
            sink.success(Cell::new());
        });
    }

    fn list(
        &mut self,
        ctx: RpcContext,
        _req: ListCellsRequest,
        sink: UnarySink<ListCellsResponse>,
    ) {
        ctx.spawn(async move {
            sink.success(ListCellsResponse::new());
        });
    }

    fn execute(&mut self, ctx: RpcContext, _req: ExecuteCellRequest, sink: UnarySink<Empty>) {
        ctx.spawn(async move {
            sink.success(Empty::new());
        });
    }
}

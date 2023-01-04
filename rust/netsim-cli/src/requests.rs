// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate frontend_proto;

use frontend_proto::frontend;

pub fn get_version(mut writer: impl std::io::Write) {
    let result = frontend::VersionResponse::new();
    // TODO: Update VersionResponse with actual response from frontend-client-cxx
    let result_json = serde_json::to_string(&result).unwrap();
    writeln!(writer, "Netsim VersionResponse json string {}", result_json)
        .expect("Error printing netsim version.");
}

#[test]
fn test_get_version() {
    let mut res = Vec::new();
    get_version(&mut res);
    assert_eq!(res, b"Netsim VersionResponse json string {\"version\":\"\"}\n")
}

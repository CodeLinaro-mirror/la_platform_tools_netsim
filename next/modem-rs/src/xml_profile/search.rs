// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use super::schema::*;
use crate::sim_service::{EF_ICCID_ID, EF_IMSI_ID};

fn find_in_df<T, F>(df: &XmlDedicatedFile, file_id: u16, extractor: &F) -> Option<T>
where
    F: Fn(&XmlElementaryFileMember) -> Option<T>,
{
    df.members.iter().find_map(|member| match member {
        XmlDedicatedFileMember::Elementary(ef) => {
            if ef.id == file_id {
                ef.members.iter().find_map(extractor)
            } else {
                None
            }
        }
        XmlDedicatedFileMember::Dedicated(sub_df) => find_in_df(sub_df, file_id, extractor),
        XmlDedicatedFileMember::ApplicationDedicated(adf) => find_in_adf(adf, file_id, extractor),
    })
}

fn find_in_adf<T, F>(adf: &XmlApplicationDedicatedFile, file_id: u16, extractor: &F) -> Option<T>
where
    F: Fn(&XmlElementaryFileMember) -> Option<T>,
{
    adf.members.iter().find_map(|member| match member {
        XmlApplicationDedicatedFileMember::ElementaryFile(ef) => {
            if ef.id == file_id {
                ef.members.iter().find_map(extractor)
            } else {
                None
            }
        }
        XmlApplicationDedicatedFileMember::DedicatedFile(df) => find_in_df(df, file_id, extractor),
        _ => None,
    })
}

pub fn find_ccid(mf: &XmlDedicatedFile) -> Option<String> {
    find_in_df(mf, EF_ICCID_ID, &|member| match member {
        XmlElementaryFileMember::Ccid(ccid) => Some(ccid.clone()),
        _ => None,
    })
}

pub fn find_imsi(
    adfs: &[XmlApplicationDedicatedFile],
    mf: Option<&XmlDedicatedFile>,
) -> Option<String> {
    let extractor = |member: &XmlElementaryFileMember| match member {
        XmlElementaryFileMember::Cimi(cimi) => Some(cimi.clone()),
        _ => None,
    };
    for adf in adfs {
        if let Some(imsi) = find_in_adf(adf, EF_IMSI_ID, &extractor) {
            return Some(imsi);
        }
    }
    mf.and_then(|df| find_in_df(df, EF_IMSI_ID, &extractor))
}

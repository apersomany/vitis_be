use std::{collections::HashSet, ops::SubAssign, sync::Arc, time::Duration};

use anyhow::{anyhow, Error};
use axum::{
    extract::{Query, State},
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use tokio::sync::broadcast;
use vitis_be_macros::macroql;

use crate::{
    states::{
        series::{Image, Single, Ticket, Viewer, KHTML},
        States,
    },
    util::{get_param, iso, now, spawn_solo},
};

use super::Result;

#[derive(Deserialize)]
pub struct SingleReq {
    series_id: i64,
    single_id: i64,
    #[serde(default)]
    wait_free: bool,
    #[serde(default)]
    free: bool,
}

#[derive(Serialize)]
pub struct SingleRes {
    meta: Single,
}

macroql! {
    query ticket_check (
        seriesId: Long,
    ) {
        contentCheckFreeTicket(seriesId) {
            list: [] {
                count: Long
            }
        }
    }
}

macroql! {
    query ticket_ready (
        seriesId: Long,
        productId: Long,
        from: QueryFromPage,
        nonstopWatching: Boolean,
        pickExactly: Boolean,
        popupOn: Boolean,
        includeWaitfree: Boolean
    ) {
        contentMyTicket(seriesId, includeWaitfree) {
            ticketOwnCount: Long,
            ticketRentalCount: Long,
            waitfree: ? {
                chargedAt: String
            }
        },
        readyToUseTicket(
            seriesId,
            productId,
            from,
            nonstopWatching,
            pickExactly,
            popupOn
        ) {
            process: String,
            available: ? {
                ticketOwnType: String?,
                ticketRentalType: String?
            }
        }
    }
}

macroql! {
    query my_tickets (
        seriesId: Long,
        includeWaitfree: Boolean
    ) {
        contentMyTicket(seriesId, includeWaitfree) {
            ticketOwnCount: Long,
            ticketRentalCount: Long,
            waitfree: ? {
                chargedAt: String
            }
        }
    }
}

async fn use_ticket(
    states: &Arc<States>,
    account_id: i64,
    series_id: i64,
    single_id: i64,
    ticket_type: impl ToString,
) -> Result<()> {
    macroql! {
        mutation use_ticket (
            input: TicketUseMutationInput {
                productId: Long,
                ticketType: String
            }
        ) {
            useTicket(input) {
                waitfreeChargedAt: String?
            }
        }
    }
    let sels = use_ticket(
        states.get_acc(account_id)?.client(),
        use_ticket::Vars {
            input: use_ticket::vars::TicketUseMutationInput {
                product_id: single_id,
                ticket_type: ticket_type.to_string(),
            },
        },
    )
    .await?;
    if let Some(wait_free) = sels.use_ticket.waitfree_charged_at {
        states.get_srs(series_id)?.get_tkt(account_id)?.wait_free = iso(&wait_free)?;
    }
    Ok(())
}

async fn get_single(
    states: &Arc<States>,
    client: Client,
    series_id: i64,
    single_id: i64,
) -> Result<Json<SingleRes>> {
    macroql! {
        query viewer (
            seriesId: Long,
            productId: Long
        ) {
            viewerInfo(seriesId, productId) {
                item {
                    title: String
                },
                viewerData {
                    ... ImageViewerData {
                        imageDownloadData {
                            files: [] {
                                size: Long,
                                secureUrl: String,
                            }
                        }
                    },
                    ... TextViewerData {
                        contentsList: [] {
                            chapterId: Long,
                            contentId: Long,
                            secureUrl: String
                        }
                    }
                },
                prevItem: ? {
                    productId: Long,
                },
                nextItem: ? {
                    productId: Long,
                }
            }
        }
    }
    use viewer::sels::viewer_info::ViewerData;
    let sels = viewer(
        client,
        viewer::Vars {
            series_id,
            product_id: single_id,
        },
    )
    .await?;
    let single = Single {
        title: sels.viewer_info.item.title,
        viewer: match sels.viewer_info.viewer_data {
            ViewerData::ImageViewerData {
                image_download_data,
            } => {
                let mut images = Vec::new();
                for file in image_download_data.files {
                    images.push(Image {
                        size: file.size,
                        kid: get_param(&file.secure_url, "kid")?,
                    })
                }
                Viewer::ImageList(images)
            }
            ViewerData::TextViewerData { contents_list } => {
                let mut khtmls = Vec::new();
                for content in contents_list {
                    khtmls.push(KHTML {
                        chapter_id: content.chapter_id,
                        content_id: content.content_id,
                        kid: get_param(&format!("kid={}", content.secure_url), "kid")?,
                    })
                }
                Viewer::KakaoHTML(khtmls)
            }
            ViewerData::Unknown => Err(anyhow!("unsupported single type"))?,
        },
        prev: sels.viewer_info.prev_item.map(|e| e.product_id),
        next: sels.viewer_info.next_item.map(|e| e.product_id),
    };
    states
        .get_srs(series_id)?
        .single_map
        .insert(single_id, single.clone());
    Ok(Json(SingleRes { meta: single }))
}

async fn finder_job(
    state: Arc<States>,
    updated: HashSet<i64>,
    series_id: i64,
    single_id: i64,
) -> Result<bool> {
    let find_channel = state.find_map.get(&series_id).map(|e| e.resubscribe());
    let mut find_channel = if let Some(find_channel) = find_channel {
        find_channel
    } else {
        let (find_tx, find_rx) = broadcast::channel(1024);
        spawn_solo(async move {
            state.find_map.insert(series_id, find_tx.subscribe());
            let all_accounts = state
                .accounts
                .iter()
                .map(|e| *e.key())
                .collect::<Vec<i64>>();
            for account_id in all_accounts {
                if !updated.contains(&account_id) {
                    if !state
                        .get_srs(series_id)?
                        .ticket_map
                        .contains_key(&single_id)
                    {
                        ticket_check(
                            state.get_acc(account_id)?.client(),
                            ticket_check::Vars { series_id },
                        )
                        .await?;
                    };
                    let sels = my_tickets(
                        state.get_acc(account_id)?.client(),
                        my_tickets::Vars {
                            series_id,
                            include_waitfree: true,
                        },
                    )
                    .await?;
                    let wait_free = if let Some(wait_free) = sels.content_my_ticket.waitfree {
                        iso(&wait_free.charged_at)?
                    } else {
                        i64::MAX
                    };
                    let permanent = sels.content_my_ticket.ticket_rental_count
                        + sels.content_my_ticket.ticket_own_count
                        - if now() > wait_free { 1 } else { 0 };
                    if permanent > 0 {
                        find_tx.send(true)?;
                    }
                    let series = state.get_srs(series_id)?;
                    series.ticket_map.insert(account_id, Ticket::default());
                    let mut ticket = series.get_tkt(account_id)?;
                    ticket.permanent = permanent;
                    ticket.wait_free = wait_free;
                }
            }
            spawn_solo(async move {
                for _ in 0..3600 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    find_tx.send(false).unwrap();
                }
                state.find_map.remove(&series_id);
            });
            Ok::<(), Error>(())
        });
        find_rx
    };
    Ok(find_channel.recv().await?)
}

pub async fn single(
    State(state): State<Arc<States>>,
    Query(query): Query<SingleReq>,
) -> Result<Json<SingleRes>> {
    let SingleReq {
        series_id,
        single_id,
        wait_free,
        free,
    } = query;
    spawn_solo(async move {
        let single = state
            .get_srs(series_id)?
            .single_map
            .get(&single_id)
            .map(|e| e.clone());
        if let Some(single) = single {
            Ok(Json(SingleRes { meta: single }))
        } else {
            if free {
                return get_single(&state, state.client.clone(), series_id, single_id).await;
            }
            for i in 0..2 {
                if wait_free {
                    let wait_frees = state
                        .get_srs(series_id)?
                        .ticket_map
                        .iter_mut()
                        .filter_map(|e| {
                            if now() > e.wait_free && e.wait_free > 0 {
                                Some(*e.key())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<i64>>();
                    for account_id in wait_frees {
                        if use_ticket(&state, account_id, series_id, single_id, "RentWaitFree")
                            .await
                            .is_ok()
                        {
                            return get_single(
                                &state,
                                state.get_acc(account_id)?.client(),
                                series_id,
                                single_id,
                            )
                            .await;
                        }
                    }
                }
                let mut updated = HashSet::new();
                let permanents = state
                    .get_srs(series_id)?
                    .ticket_map
                    .iter_mut()
                    .filter_map(|e| {
                        if e.permanent > 0 {
                            Some(*e.key())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                for account_id in permanents {
                    let mut sels = ticket_ready(
                        state.get_acc(account_id)?.client(),
                        ticket_ready::Vars {
                            series_id,
                            product_id: single_id,
                            from: "Viewer".to_string(),
                            nonstop_watching: false,
                            pick_exactly: true,
                            popup_on: false,
                            include_waitfree: true,
                        },
                    )
                    .await?;
                    match sels.ready_to_use_ticket.process.as_str() {
                        "ForceUseRentalTicket" | "AskTicketChoice" => {
                            if let Some(available) = sels.ready_to_use_ticket.available {
                                if let Some(ticket_type) = available.ticket_rental_type {
                                    use_ticket(
                                        &state,
                                        account_id,
                                        series_id,
                                        single_id,
                                        ticket_type,
                                    )
                                    .await?;
                                    sels.content_my_ticket.ticket_rental_count.sub_assign(1);
                                }
                            }
                        }
                        "ForceUseOwnTicket" => {
                            if let Some(available) = sels.ready_to_use_ticket.available {
                                if let Some(ticket_type) = available.ticket_own_type {
                                    use_ticket(
                                        &state,
                                        account_id,
                                        series_id,
                                        single_id,
                                        ticket_type,
                                    )
                                    .await?;
                                    sels.content_my_ticket.ticket_own_count.sub_assign(1);
                                }
                            }
                        }
                        "AlreadyConfirmed" => {}
                        unknown => {
                            Err(anyhow!("unknown process: \"{unknown}\""))?;
                        }
                    }
                    let wait_free = if let Some(wait_free) = sels.content_my_ticket.waitfree {
                        iso(&wait_free.charged_at)?
                    } else {
                        i64::MAX
                    };
                    let series = state.get_srs(series_id)?;
                    let mut ticket = series.get_tkt(account_id)?;
                    ticket.permanent = sels.content_my_ticket.ticket_rental_count
                        + sels.content_my_ticket.ticket_own_count
                        - if now() > wait_free { 1 } else { 0 };
                    ticket.wait_free = wait_free;
                    drop(ticket);
                    drop(series);
                    updated.insert(account_id);
                    return get_single(
                        &state,
                        state.get_acc(account_id)?.client(),
                        series_id,
                        single_id,
                    )
                    .await;
                }
                if i == 0 {
                    if !finder_job(state.clone(), updated, series_id, single_id).await? {
                        Err(anyhow!("ticket finder job is on a cooldown"))?
                    }
                }
                if i == 1 {}
            }
            Err(anyhow!("not enough tickets"))?
        }
    })
    .await?
}

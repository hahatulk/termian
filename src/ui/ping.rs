use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use gpui::{
    div, percentage, prelude::*, px, rgb, rgba, svg, Animation, AnimationExt, AnyElement, Context,
    Div, SharedString, Stateful, Transformation,
};
use termian_native::ping::{self, Reply};
use termian_native::theme;
use termian_native::vault::{Host, HostId};

use super::assets::icon;
use super::cloud::Tooltip;
use super::motion;
use super::pointer::PointerFeedback;
use crate::Shell;

const CONCURRENT: usize = 8;
const SCAN: u32 = 0x66dcff;

#[derive(Clone, Copy)]
enum Status {
    Queued,
    Running,
    Done(Reply),
}

struct Request {
    id: HostId,
    address: String,
    ticket: u64,
}

struct Entry {
    address: String,
    ticket: u64,
    status: Status,
}

#[derive(Default)]
pub(crate) struct HostPings {
    entries: HashMap<HostId, Entry>,
    queue: VecDeque<Request>,
    active: usize,
    ticket: u64,
    batch: bool,
}

impl HostPings {
    fn enqueue(&mut self, host: &Host) {
        let address = host.address.trim();
        if address.is_empty() || matches!(self.status(host), Some(Status::Queued | Status::Running))
        {
            return;
        }
        self.ticket += 1;
        let ticket = self.ticket;
        self.entries.insert(
            host.id,
            Entry {
                address: address.to_owned(),
                ticket,
                status: Status::Queued,
            },
        );
        self.queue.push_back(Request {
            id: host.id,
            address: address.to_owned(),
            ticket,
        });
    }

    fn status(&self, host: &Host) -> Option<Status> {
        self.entries
            .get(&host.id)
            .filter(|entry| entry.address == host.address.trim())
            .map(|entry| entry.status)
    }

    fn entry(&mut self, request: &Request) -> Option<&mut Entry> {
        self.entries
            .get_mut(&request.id)
            .filter(|entry| entry.ticket == request.ticket)
    }

    fn pending(&self) -> usize {
        self.active + self.queue.len()
    }
}

fn radar() -> AnyElement {
    let ring = || {
        div()
            .absolute()
            .rounded_full()
            .border_1()
            .border_color(rgb(SCAN))
    };
    let arc = svg()
        .path(icon("spinner_arc"))
        .size(px(16.))
        .text_color(rgb(SCAN));
    let animation = if motion::enabled() {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(ring().with_animation(
                "ping-wave",
                Animation::new(Duration::from_millis(1000)).repeat(),
                |d, t| {
                    let size = 8. + 14. * t;
                    d.size(px(size))
                        .top(px((24. - size) / 2.))
                        .left(px((24. - size) / 2.))
                        .opacity(0.8 * (1. - t))
                },
            ))
            .child(arc.with_animation(
                "ping-sweep",
                Animation::new(Duration::from_millis(800)).repeat(),
                |arc, t| arc.with_transformation(Transformation::rotate(percentage(t))),
            ))
            .into_any_element()
    } else {
        div()
            .absolute()
            .inset_1()
            .rounded_full()
            .border_1()
            .border_color(rgb(SCAN))
            .into_any_element()
    };
    div()
        .size(px(24.))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .child(animation)
        .child(div().size(px(5.)).rounded_full().bg(rgb(SCAN)))
        .into_any_element()
}

impl Shell {
    fn start_pings(&mut self, id: Option<HostId>, cx: &mut Context<Self>) {
        if let Some(id) = id {
            if let Some(host) = self.vault.host(id) {
                self.host_pings.enqueue(host);
            }
        } else if !self.host_pings.batch {
            let ids: HashSet<_> = self.vault.hosts.iter().map(|host| host.id).collect();
            self.host_pings.entries.retain(|id, _| ids.contains(id));
            for host in &self.vault.hosts {
                self.host_pings.enqueue(host);
            }
            self.host_pings.batch = self.host_pings.pending() > 0;
        }
        self.dispatch_pings(cx);
    }

    fn dispatch_pings(&mut self, cx: &mut Context<Self>) {
        while self.host_pings.active < CONCURRENT {
            let Some(request) = self.host_pings.queue.pop_front() else {
                break;
            };
            let current = self
                .vault
                .host(request.id)
                .is_some_and(|host| host.address.trim() == request.address);
            let Some(entry) = self.host_pings.entry(&request) else {
                continue;
            };
            if !current {
                self.host_pings.entries.remove(&request.id);
                continue;
            }
            entry.status = Status::Running;
            self.host_pings.active += 1;
            let address = request.address.clone();
            let work = cx.background_spawn(async move { ping::check(&address) });
            cx.spawn(async move |this, cx| {
                let start = Instant::now();
                let reply = work.await;
                if motion::enabled() {
                    let remaining = Duration::from_millis(450).saturating_sub(start.elapsed());
                    if !remaining.is_zero() {
                        cx.background_executor().timer(remaining).await;
                    }
                }
                let _ = this.update(cx, |shell, cx| {
                    shell.host_pings.active -= 1;
                    let current = shell
                        .vault
                        .host(request.id)
                        .is_some_and(|host| host.address.trim() == request.address);
                    if let Some(entry) = shell.host_pings.entry(&request) {
                        if current {
                            entry.status = Status::Done(reply);
                        } else {
                            shell.host_pings.entries.remove(&request.id);
                        }
                    }
                    shell.dispatch_pings(cx);
                });
            })
            .detach();
        }
        if self.host_pings.pending() == 0 {
            self.host_pings.batch = false;
        }
        cx.notify();
    }

    pub(super) fn ping_all_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let waiting = self.host_pings.batch;
        let enabled = !waiting && !self.vault.hosts.is_empty();
        let tooltip = if waiting {
            format!("Pinging hosts ({})", self.host_pings.pending())
        } else {
            "Ping all saved hosts".to_owned()
        };
        div()
            .id("ping-all-hosts")
            .h(px(26.))
            .px_1()
            .gap_1()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .when(enabled, |d| {
                d.surface_feedback(theme::INPUT)
                    .on_click(cx.listener(|shell, _, _, cx| shell.start_pings(None, cx)))
            })
            .when(!enabled && !waiting, |d| d.opacity(0.5))
            .when(waiting, |d| d.child(radar()))
            .when(!waiting, |d| {
                d.child(
                    div()
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            svg()
                                .path(icon("ping"))
                                .size(px(15.))
                                .text_color(rgb(theme::TEXT)),
                        ),
                )
            })
            .when(self.cards_width.get() >= 450., |d| {
                d.pr_2()
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT))
                    .child("PING ALL")
            })
            .tooltip(move |_, cx| cx.new(|_| Tooltip(tooltip.clone())).into())
    }

    pub(super) fn ping_dot(&self, host: &Host, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = host.id;
        let status = self.host_pings.status(host);
        let waiting = matches!(status, Some(Status::Queued | Status::Running));
        let (color, text) = match status {
            None => (theme::TEXT_MUTED, "Ping host".to_owned()),
            Some(Status::Queued) => (0xffbb73, "Waiting to ping".to_owned()),
            Some(Status::Running) => (SCAN, "Pinging host".to_owned()),
            Some(Status::Done(Reply::Echo(ms))) => (
                theme::ACCENT,
                ms.map_or_else(
                    || "Ping replied. Click to ping again".to_owned(),
                    |ms| format!("Ping: {ms:.1} ms. Click to ping again"),
                ),
            ),
            Some(Status::Done(Reply::NoReply)) => {
                (theme::ERROR, "No ping reply. Click to try again".to_owned())
            }
            Some(Status::Done(Reply::Unresolved)) => (
                theme::ERROR,
                "Address could not be resolved. Click to try again".to_owned(),
            ),
            Some(Status::Done(Reply::Unavailable)) => (
                theme::ERROR,
                "Ping unavailable. Click to try again".to_owned(),
            ),
        };
        div()
            .id(SharedString::from(format!("host-ping-{id}")))
            .size(px(24.))
            .flex_none()
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .when(!waiting, |d| d.icon_feedback())
            .when(matches!(status, Some(Status::Running)), |d| {
                d.child(radar())
            })
            .when(!matches!(status, Some(Status::Running)), |d| {
                d.child(
                    div()
                        .size(px(7.))
                        .rounded_full()
                        .bg(rgb(color))
                        .shadow(vec![gpui::BoxShadow {
                            color: rgba((color << 8) | 0x44).into(),
                            blur_radius: px(4.),
                            offset: gpui::point(px(0.), px(0.)),
                            spread_radius: px(0.),
                        }]),
                )
            })
            .tooltip(move |_, cx| cx.new(|_| Tooltip(text.clone())).into())
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                if !waiting {
                    shell.start_pings(Some(id), cx);
                }
            }))
    }
}

# ADR-0024 — Kubernetes platform on Proxmox

- **Status**: Proposed. Thibault chose every recommendation on October 31
  (SOB-175). The ADR becomes Accepted once he confirms the server facts of
  §9 (SOB-181) and approves the PR.
- **Date**: 2026-10-31
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Infrastructure project (SOB-72)
- **Amendment 1** (2026-10-31, SOB-218): the shared host, a smaller starting cap, a shared front end and the portability rule. See the section below.
- **Related**: ADR-0013 (team server), ADR-0014 (managed cloud, same binary,
  Amendment 1), ADR-0015 and ADR-0016 (privacy), ADR-0021 (one app, served
  by the server), ADR-0022 (one database per space, accounts database);
  SOB-73 (infrastructure as code and deployment) builds on this one

> **Summary.** Sobr runs on Kubernetes in virtual machines on Thibault's
> Proxmox server.
>
> - **Nodes:** Talos Linux, whose system is read-only and changed only
>   through its API from the infrastructure code.
> - **Network:** Cilium gives the network, the network policies and the
>   entry (Gateway API), and cert-manager gets the certificates.
> - **Functions:** Knative Serving scales them to zero.
> - **Managed cloud:** the Sobr server runs as ADR-0022 sets it, as one
>   deployment that serves many spaces, each in its own database.
> - **Entry from the internet:** a small EU virtual server that holds no
>   secret and forwards encrypted traffic to the cluster, so Sobr does not
>   depend on the home line's address.
> - **Customer data:** four layers protect it. A continuous, encrypted copy
>   of every database goes to an EU object store, from which the server
>   restores itself. etcd snapshots also leave the machine. Proxmox can
>   roll VMs back. Restores are tested monthly, and the full rebuild before
>   launch.
> - **Disk keys:** they come from a key server on a small machine that
>   Thibault controls elsewhere, which later holds the third quorum vote.
> - **CI runners:** throwaway VMs on a separate network.
>
> The managed cloud opens, trial included, only once these protections,
> erasure, export and the GDPR documents exist. A second failure domain is
> needed before the public launch.

## Points decided

Thibault answered these ten points on October 31 in the Decision issue
SOB-175, and chose the recommendation of each. Each choice rests on the
matching part of the Decision section, which compares the options with their
cost, effort and risk. The facts about the server in §9 are still to be
confirmed, in SOB-181.

1. **Kubernetes distribution** (§1). Chosen: Talos Linux, configured
   from code, with no SSH. Alternative: k3s on a regular Linux that must be
   patched and kept in line by hand.
2. **Serverless runtime** (§2). Chosen: Knative Serving, with Kourier
   as its internal router. Alternative: KEDA, whose scale-to-zero for HTTP
   is a beta add-on.
3. **Storage** (§3). Chosen: local volumes, with the server restoring
   itself from its database copies when its volume is lost. Volumes are
   replicated only between machines at the same site. Alternative: Longhorn
   from the start.
4. **Network** (§4). Chosen: Cilium, including its Gateway API.
   Alternative: Flannel and Traefik, with no network policies.
5. **Entry from the internet** (§4). Chosen: a small EU virtual server
   in front. It holds no secret and forwards Sobr's encrypted traffic over
   WireGuard. Sobr then needs no fixed home address, and the other site on
   the home line stays as it is. Alternative: the home address directly,
   with a hostname router that moves the existing nginx. Never a proxy
   outside the EU that decrypts the traffic.
6. **Managed-cloud layout** (§5). Chosen: as ADR-0022 sets, one
   deployment of the Sobr server for all spaces, with the accounts database.
   Alternative: one deployment per organization, which needs a separate
   accounts service and an amendment of ADR-0022.
7. **CI runners** (§5). Chosen: throwaway VMs, rebuilt for each job, on
   a separate network that cannot reach the cluster, the backups or the
   Proxmox interface. Alternative: runners as pods in the cluster.
8. **When the managed cloud opens** (§6). The trial needs a card and turns
   into a paid plan (ADR-0014 Amendment 1), so the gate applies to the
   trial. Chosen: open once backups, monitoring, encrypted disks, a
   timed restore, erasure, export and the GDPR documents are in place, with
   a second failure domain before the public launch of March 2027.
   Alternatives: open on one machine now, with best effort in the contract;
   or wait for the second failure domain before opening anything.
9. **Backups** (§6). Chosen: four layers.
   - A continuous, encrypted copy of every database to an EU object store
     that keeps copies in three zones, as ADR-0014's feature table promises.
   - etcd snapshots off the machine.
   - Proxmox VM rollback.
   - Monthly restore tests and a full rebuild test, plus an erasure log
     replayed after any restore.

   Alternative: nightly VM backups only.
10. **Disk keys and the third node** (§6). Chosen: the nodes get their
    disk keys from a key server on a small machine that Thibault controls,
    at another place. The same machine later holds the third quorum vote.
    After a long outage, such as a power cut, a move or a theft, it waits
    for Thibault's confirmation before it serves keys again. Its own disk
    is encrypted, and its keys have offline copies. Alternatives: a rented
    EU virtual server, whose provider could read the keys and etcd; keys
    derived on the host, so a stolen machine unlocks itself; or a
    passphrase typed at each boot.

## Amendment 1 — The shared host, a smaller starting cap and the portability rule (2026-10-31)

**Status.** In force when the PR is merged (SOB-218, SOB-87: amendment by PR); where it conflicts with the original text, the amendment wins. The ADR itself stays Proposed until Thibault confirms the facts of A1 that are still open. The original text is kept, and a note under each heading marks what changes. The sections of this amendment are numbered A1 to A7, and "§" points to the sections of the original.

**Why.** This ADR was written before the survey of the host (SOB-181). The host is shared with another project's production, and Thibault decided that Sobr may run everything on it with one rule, portability (SOB-211). He set the scope of Claude's approvals (SOB-212), the split with BrilliantStudio's repo and the shared front end (SOB-213), and a starting cap for Sobr. The layout of the host, its ranges, limits and rules are in the host contract, which is private because it describes a private host. ADR-0026 holds how Sobr deploys.

### A1. What the survey changes

The details of the host are in the private contract and in SOB-181. This ADR keeps what its decisions need:

- **The host is shared** with another project's production, which has priority. Host-level changes need Thibault's explicit go.
- **Sobr's starting cap** is 10 GB of memory, 4 vCPU and 200 GB of disk, in total (SOB-213 and the draft of the host contract). The 64 to 80 GB of §8 do not fit.
- **No redundancy is assumed** for the host's disk or its power, and no spare machine until §7's second failure domain exists.
- **The public address is shared and not fixed,** and only some ports can be forwarded at home. The default KubeSpan port is outside them (A4).
- **Open facts** that Thibault still has to confirm (SOB-181): upload speed, a spare disk, backup power, and a third place for the small key machine of §6. ECC memory has been checked and is absent.

### A2. The ten points against the facts

| Point | Holds? | What changes, or the risk accepted |
|--|--|--|
| 1 Talos | Yes | One small node (A3). Its VMs are created in the Sobr pool with the token of ADR-0026, which has no host-level privilege |
| 2 Knative | Yes | Its memory cost (a few hundred MB) matters under the cap |
| 3 Local volumes | Yes | The token that powers a VM off (§3) is limited to Sobr's VMs |
| 4 Network, Cilium, certificates | Yes | The nodes sit on a Sobr bridge created once by Thibault (host contract) |
| 5 Entry through an EU front server | Yes, and it is shared | One small EU server is a layer-4 relay on the TLS host name (SNI), with no TLS termination: each project keeps its certificates and keys at home. Each project has its own tunnel, keys and routes, and an unknown host name is refused. The tunnels start from home, so no new port is opened. The text "the other site keeps the home address and its nginx" is out of date. A routing change goes through a PR read by both projects and validated by Thibault; if that is not accepted, there are two front ends (SOB-213) |
| 6 Layout of the services | Yes | None |
| 7 CI runners | Yes, but not yet | The design holds: isolated throwaway VMs with no route to the management network. The cap leaves no room for one, so CI stays on GitHub's runners until the cap rises or a second machine exists (SOB-75) |
| 8 The cloud opens after its protections | Yes | "A UPS covers short power cuts" is not assumed. The risk is in A6 |
| 9 Four backup layers | Yes, layer 3 changes | Layers 1 and 2 are off the machine and hold. The host's own backup server is not a Sobr layer, so layer 3 (rollback copies on another disk) is replaced by a copy in a third place. A restore test somewhere other than the host proves portability (A5, P3) |
| 10 Disk keys from a key server | Yes | The third place is still open (A1) |

### A3. Sizing under the cap

§8 adds up to 28 vCPU and 64 to 80 GB, plus mirrored disks ("Hypervisor and ZFS cache" and "1 TB of mirrored disk" no longer apply). Under the cap of 10 GB, 4 vCPU and 200 GB:

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — One small Talos node that holds the control plane and the workloads (8 GB, 3 vCPU, about 150 GB), plus the router VM (2 GB, 1 vCPU); monitoring kept to metrics with a short retention (recommended)** | No high availability and no second worker: an upgrade of the node stops the lab for its duration | Small | Medium: it fits the cap exactly, with no room for growth or for CI. It is a lab, not the managed cloud |
| B — Three very small nodes (about 3 GB each) and the router | A three-member etcd | Medium | 3 GB cannot hold a control plane, Cilium and Knative together; the overhead of three nodes eats the cap for no gain on one machine |
| C — Keep §8 and raise the cap first | Memory to buy or a second machine | Small | The launch waits |

The managed cloud does not open on this cap: §6 already makes its opening depend on its protections and on a second failure domain (§7), and the cap must rise before then. The figures are starting figures, as §8 says. The cap, the limits and the pinning belong to the host contract, and the daily check of ADR-0026 compares the real machines with them.

### A4. The mesh port and the front-end link

§1 uses Talos KubeSpan only for a machine at another site, and §4 forwards Sobr's traffic over WireGuard from the front server to the cluster. The default KubeSpan port, UDP 51820, is outside the ports that can be forwarded at home.

- **The front-end link needs no inbound port at home.** The router VM dials out to the front server, which listens on a public port of its own, with a keepalive. WireGuard carries traffic both ways once the tunnel is up.
- **KubeSpan between sites** may need an inbound port at home. Whether KubeSpan accepts another port is not stated on the pages read; it is a fact to verify before a second site (§7) depends on it. IPv6, available at home, is a third path to test.

### A5. The rule of portability (SOB-211)

Nothing of Sobr may depend on this particular host. The rule has five parts, each with its check:

| # | Rule | Check |
|--|--|--|
| P1 | All infrastructure is in code | A rebuild from the repo on an empty host reproduces the platform (the full rebuild test of §6) |
| P2 | No address, host name, VMID or storage ID is written by hand outside the code | A check in `infra` fails on one outside the host file |
| P3 | Backups restore on another machine and are kept off the host | The monthly restore into a spare environment elsewhere than the host, and the rebuild on a second machine before launch |
| P4 | No feature that only this box offers | A review refuses it |
| P5 | Host-specific values are variables in one file per host | `infra` plans against a second host file without error |

### A6. Risks accepted until the host improves

Thibault accepted the host's risks on October 31 (SOB-211, private). The design draws these consequences:

- **One failure domain** until §7's second one exists, and no redundancy of the host's disk or power is assumed.
- **After a power cut,** the nodes need the key server and the line to unlock their disks on restart, and the key machine's own disk needs a typed passphrase (§6). A long outage while Thibault is away leaves the lab down. The offline copies of the keys (§6) and a runbook (ADR-0026 issue 10) are the break-glass path. §6 lists the conditions for opening the cloud: this risk joins them, for Thibault to accept again at launch.
- **The line's terms of use** are an accepted risk recorded in SOB-211, until a business line exists.

### A7. Who approves

The sentence of the Context that says deployments follow Thibault's approval from his GitHub account is replaced by ADR-0026 and SOB-212: Claude may approve and apply changes to what Sobr owns, and anything on the host or touching BrilliantStudio needs Thibault's explicit go.

**Issues.** SOB-218, SOB-181, SOB-175, SOB-211, SOB-212, SOB-213, SOB-217.

## Context and problem statement

> Amendment 1 (2026-10-31): the host is shared with another project's production, it is not an Ubuntu machine with nginx, the address is shared and not fixed (A1), and the sentence on approvals is replaced (A7).


On October 30 Thibault decided that his dedicated Proxmox server hosts every
Sobr service: CI builds, the organization server, the app on the web, the
site, routines and the managed cloud. Functions are serverless and scale on
demand, and the platform must grow to more machines later. The
infrastructure is described as code in its own repo, and it deploys after a
merge to `main`, only once Thibault approves from his GitHub account. SOB-73
covers that flow.

What exists today:

- **The organization server** is one Rust binary that stores its data in
  SQLite with WAL.
  - Its guide asks for one process per data folder, and warns that WAL does
    not work well on a shared network file system
    (`docs/operations/team-aggregator.md`).
  - It serves HTTPS itself, with a self-signed certificate (`deploy/team/`).
  - ADR-0022 gives it one database per space and an accounts database, and
    lets one server serve many spaces.
  - ADR-0021 makes it serve the app on the web, on the same origin as its
    API.
- **Packaging:** a Dockerfile and a Docker Compose file exist
  (`deploy/team/`), but no image is published yet. The Cloud and
  self-hosting project owns the published image and the Helm chart, and this
  platform reuses them.
- **The site** is a static build. Today it is copied with `rsync` to an
  Ubuntu machine whose nginx serves another site on ports 80 and 443 of the
  same public address (`docs/operations/site-deploy.md`).
- **Customer data** in the managed cloud is the use data of ADR-0022:
  metrics by day, member IDs, and no prompt text. It is small, one SQLite
  file per space, but it is personal data under the GDPR.
- **The decisions of October 30** in
  [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD)
  say: hosting in Europe; session detail kept 24 months; self-hosting with
  Docker Compose or Helm.
- **ADR-0014** requires, for any Sobr cloud offer:
  - EU hosting (it gives OVH, Scaleway and Clever Cloud data centres as
    examples), a named DPO, standard contract clauses and a public privacy
    policy;
  - a documented DPIA, erasure and self-service export, all implemented
    (condition 4);
  - per its feature table, backups in three EU zones for the managed cloud.

  A server in France is EU hosting. What it lacks next to a data centre is
  redundant power, network and physical security, which is why this ADR
  asks for a second failure domain before the public launch (§6).
- **Builds:** macOS builds cannot run on Proxmox, because Apple's license
  allows macOS only on Apple hardware, so they stay on GitHub's runners. The
  repos are public, so a pull request from a fork must never run on a
  self-hosted runner (SOB-75).

The public address is known, but not whether it is fixed, nor the
hardware, the line or the power. The choices below do not depend on exact
figures; the sizing in §8 does. §9 lists what Thibault should confirm.

### Out of scope

- Installing anything (the issues listed at the end).
- The infrastructure-as-code tools and the deployment flow: SOB-73.
- The Sobr server's image, Helm chart and setup wizard, and how
  organizations are addressed (one hostname or subdomains): Cloud and
  self-hosting project. This ADR states only what the platform needs from
  them.
- Accounts, purchases, domains and contracts: human-only, listed as issues.

## Decision

### 1. Kubernetes distribution, and how a machine joins

> Amendment 1 (2026-10-31): the default KubeSpan port, UDP 51820, is outside the ports that can be forwarded at home (A4), and the first cluster is one small node (A3).

**Talos Linux in Proxmox VMs.** Talos is a Linux built only to run
Kubernetes:

- its system image is read-only, and it has no shell and no SSH;
- every node is set from a machine configuration file, and changed or
  upgraded through its API (`talosctl`);
- the files live in the infrastructure repo. SOB-73 makes the code, applied
  after review, the only way to change a node; direct `talosctl` access is
  kept for that flow and for emergencies;
- it encrypts its disks with keys from a key server (§6).

**Start on one machine:** one control-plane VM and two worker VMs. While the
control plane restarts for an upgrade, which takes a few minutes, these
pause:

- the Kubernetes API;
- functions scaling up from zero;
- scheduled jobs;
- certificate renewals.

Running workloads keep serving. Three control-plane VMs on the same machine
would protect against a VM failure, not a machine failure, so they are not
worth their memory yet.

**A second machine joins** in these steps:

1. Proxmox is installed on it, as an independent host.
2. Its VMs are created from the same code.
3. Each VM receives its Talos configuration, as a worker or a control-plane
   node, and joins the cluster.
4. If the machine is at another site, Talos's built-in WireGuard mesh
   (KubeSpan) connects the nodes. KubeSpan finds its peers through Sidero
   Labs' public discovery service, which only ever sees data encrypted by
   the nodes, plus the addresses they connect from. Hosting our own
   discovery service needs a commercial license
   ([Talos docs](https://docs.siderolabs.com/talos/v1.12/configure-your-talos-cluster/system-configuration/discovery)).

**Failover needs three failure domains.** Kubernetes keeps its state in
etcd, which needs a majority of its members to work. With two machines, the
loss of the one that holds most members stops the cluster. Real failover
therefore needs a third, small control-plane node at a third place: the
small machine of §6.

The third node holds a full copy of etcd, which includes the cluster's
secrets. Whoever controls that machine can read them. A rented server would
hand that to its provider, which is why §6 recommends a machine that
Thibault controls.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Talos Linux (chosen)** | A new tool to learn; problems are diagnosed through the API, not a shell | Medium | Low: no drift, a small attack surface, upgrades through the API, built-in mesh across sites |
| B — k3s on Debian VMs | The OS must be patched and kept in line by hand or with Ansible | Small to start, then growing | The VMs drift from the code, and SSH is one more way in |
| C — kubeadm on Debian VMs | Every component is installed and upgraded by hand | Large | More moving parts for one person to run |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Independent Proxmox hosts; Kubernetes spans them (chosen)** | Each host is managed on its own, from the same code | Small | Low: Kubernetes keeps its own quorum in etcd |
| B — A Proxmox cluster of the two hosts | A quorum device is needed to keep a two-host cluster running | Medium | A lost link or host can freeze the Proxmox side, while Kubernetes needs none of its features |

### 2. Serverless functions

**Knative Serving** runs functions as containers that scale with the
traffic, down to zero when no request comes in. Functions are for
request-driven work: webhooks from Linear, GitHub or the payment provider,
report generation, and later endpoints that see little traffic.

**Kourier** is Knative's own light router. It runs inside the cluster and
receives function traffic from the gateway (§4), so it is not exposed by
itself. Knative can also use the Gateway API directly; that path is less
mature, and can replace Kourier later.

**Scheduled work and routines:**

- scheduled work that does not touch a space database runs as Kubernetes
  CronJobs, such as a catalog refresh or a report;
- work that writes to space databases, such as retention (ADR-0022), stays
  in the Sobr server's own timers, because each data folder must have one
  application writer;
- agent routines, which run Claude, stay as Claude routines on Anthropic's
  side, as Thibault prefers: they need Claude, not this machine.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Knative Serving (chosen)** | A few hundred MB of memory for its controllers and router | Medium | Low: scale-to-zero for HTTP is its core feature |
| B — KEDA, with its HTTP add-on for scale-to-zero | A lighter core | Medium | The HTTP add-on is beta, and HTTP is our main trigger |
| C — Plain deployments with the Horizontal Pod Autoscaler | Nothing to add | Small | No scale-to-zero, against the decision of October 30 |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Kourier as Knative's internal router, behind the gateway (chosen)** | A second Envoy inside the cluster | Small | Low: Knative's default, well tested |
| B — Knative on the cluster's Gateway API (Cilium) | One router fewer | Medium | A younger integration between two projects |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — CronJobs for platform jobs, the server's timers for its databases, Claude routines for agent work (chosen)** | Three places for scheduled work, each with a clear owner | Small | Low |
| B — Everything as CronJobs in the cluster, agent routines included | One place | Medium | A second writer on space databases, and Claude credentials on the server |

### 3. Storage

> Amendment 1 (2026-10-31): the token that powers a VM off is limited to Sobr's own VMs (A2, point 3).

**Local volumes.** Each node's volumes sit on its own virtual disk, given
out by a local-path provisioner. SQLite with WAL wants a local disk, and
the Sobr server keeps its databases on one volume.

**Losing the server's node.** A local volume is tied to its node, so a
replacement pod waits until that node is fenced and the volume claim is
released. Issue 15 automates both steps:

- **Fencing:** after a few minutes without the node, its VM is powered off
  through a Proxmox API token limited to that action. A VM that hangs but
  still runs could otherwise keep writing to its databases and their copy,
  next to the restored server.
- **Release:** only then is the claim freed, and the server starts on the
  other worker with an empty volume.

On an empty volume the server first restores every database from its
continuous copy (§6), then serves. Until that automation exists, a runbook
gives the same steps to a person. The recovery time is minutes plus the
restore time, which the monthly test measures. Planned upgrades move the
server on purpose, in a maintenance window.

**Replication stays within one site.** Longhorn copies every write to each
replica before confirming it, so replicas belong on one local network.

- **A second machine at the same site:** the server's volume moves to
  Longhorn, replicated across the two machines. Longhorn gives block
  devices, on which SQLite with WAL is safe.
- **At another site:** no volume is replicated across the internet. A lost
  site is recovered by restoring from the continuous copy at the other
  site.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Local volumes with automatic restore; Longhorn only within one site (chosen)** | A few minutes of downtime per upgrade, and a restore if a node is lost | Medium (fencing and restore to automate) | Low with tested copies |
| B — Longhorn from the start, across the two worker VMs | Memory and disk for every copy; both copies on the same physical disks | Medium | Protects against the loss of a VM, not of a disk or the machine |
| C — Longhorn across two sites | Every write waits for the other site | Medium | Slow writes on home lines, and a cut link stalls the volume |

### 4. Network, entry point, certificates and exposure

> Amendment 1 (2026-10-31): the front server is shared by the two projects as a layer-4 relay, the other site is a container to be replaced, and the tunnel is dialed out from home (A2, point 5, and A4).

**Cilium** is the network of the cluster:

- it enforces network policies, which keep components apart: the Sobr
  server, functions, the site and monitoring;
- it gives load-balancer addresses on the local network (LB IPAM and L2
  announcements, which need the nodes on one bridged network);
- it implements the Gateway API, the cluster's entry.

**The Sobr server inside the cluster.** Today the server serves HTTPS with a
self-signed certificate. In the cluster:

- it serves plain HTTP that only the gateway can reach, by network policy;
- Cilium's WireGuard encryption covers traffic between nodes;
- the gateway ends TLS and adds the security headers, HSTS included;
- the Helm chart needs a plain-HTTP mode for this (Cloud and self-hosting
  project).

**Certificates.** cert-manager gets every certificate from Let's Encrypt
through DNS-01, since port 80 only redirects to HTTPS. The Cloud project
chooses how organizations are reached: one shared hostname, or one
subdomain each.

- With subdomains, only a wildcard certificate is used. One certificate per
  organization would publish every customer's name in the public
  certificate logs.
- DNS-01 needs a DNS provider with an API, which is a human-only account; an
  EU provider is preferred.

**Entry from the internet.** Sobr's names point to a small EU virtual
server:

- it reads the requested name without decrypting (SNI on port 443; the Host
  header on port 80, which only redirects to HTTPS);
- it forwards Sobr's traffic, still encrypted, over WireGuard to the
  cluster's gateway, and TLS ends on Thibault's machine;
- it holds no certificate, key or data, so losing it means a rebuild from
  code, not a breach;
- Sobr's names point to it, so Sobr needs no fixed home address, and it
  absorbs some abuse. The home address itself is already public, through
  `site-deploy.md` and the other site's DNS;
- the other site keeps the home address and its nginx, unchanged.

**Domains.** Sobr's names live under `sobr.cc` (ADR-0020):

- the domain's records move to the DNS provider with an API;
- the site's and the app's hostnames point to the front;
- the old host `sobria.brilliantstudio.co` redirects to the new one with a
  301 (ADR-0020, R1).

The registration and the DNS account are human-only (issue 11), and issue
17 sets the records.

**Logs.** The gateway and the network keep only what operations need:

- no request bodies;
- access and flow logs kept 7 days at most;
- no view of a person's connection times for anyone else (ADR-0015 rule 4).

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Cilium with its Gateway API, plus cert-manager (chosen)** | Cilium is richer, so there is more to learn | Medium | Low: network, policies, addresses and entry in one component |
| B — Flannel and Traefik, plus cert-manager | Two simple parts | Small | Flannel does not enforce network policies, so components are not kept apart |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — A small EU virtual server in front, forwarding encrypted traffic over WireGuard (chosen)** | A few euros a month, one more machine to run | Small | Low: it holds no secret, is quick to rebuild, and Sobr needs no fixed home address |
| B — The home address directly, with a hostname router at home | The existing nginx moves off ports 80 and 443 behind the router; the line needs a fixed address | Small | The home address is public, an attack hits the line directly, and the other site changes |
| C — A tunnel through a non-EU provider that ends TLS | Free | Small | That provider sees the traffic in clear and falls under non-EU law, against hosting in Europe. Ruled out |

### 5. How the Sobr services run

> Amendment 1 (2026-10-31): the CI runners' firewall also protects the other project's machines, and the cap leaves no room for them yet (A2, point 7).

| Service | How it runs |
|--|--|
| Managed cloud | One deployment of the Sobr server from the Cloud project's Helm chart, serving every organization space (one database each) and the accounts database, as ADR-0022 sets |
| App on the web | Served by the Sobr server (ADR-0021); nothing separate |
| Personal spaces, free for individuals (ADR-0013 and ADR-0014, Amendment 2) | The same deployment (ADR-0022, issue 14) |
| Site and the web demo (ADR-0017) | A small static container behind the gateway, replacing the `rsync` to nginx |
| Functions | Knative (§2) |
| Scheduled work | §2 |
| CI runners (SOB-75) | Throwaway Proxmox VMs outside the cluster, rebuilt from a clean image for each job: Linux runners, and a Windows VM for Windows builds. macOS stays on GitHub |
| Monitoring | In the cluster, plus a probe outside the machine (§6) |

**The managed cloud follows ADR-0022.** ADR-0022 gives each space its own
database and member IDs, and puts sign-in in one accounts database, so a
person joins an organization with the same account (ADR-0014 Amendment 1).
One process per organization would split that accounts database away from
the spaces and need a separate accounts service. That is an amendment of
ADR-0022, and buys little on one machine.

**CI runners never touch customer data.** They run code from pull requests,
so they live in throwaway VMs on their own network, firewalled from:

- the Talos and Kubernetes APIs;
- Proxmox Backup Server;
- the Proxmox web interface;
- the cluster's networks.

They still share the hypervisor, so a VM escape would reach it. That is why
they are rebuilt for each job and never hold secrets beyond the job's own.
The repos are public, so pull requests from forks never run on them
(SOB-75).

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — One deployment for all spaces, as ADR-0022 sets (chosen)** | One process: a crash or an upgrade touches every customer at once, for minutes | Small | Low on one machine, where everything shares one failure domain anyway |
| B — One deployment per organization | A separate accounts service and an amendment of ADR-0022; one process and one volume per organization | Large | More parts to run, for isolation that the database per space already gives at the data level |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Throwaway CI VMs on a separate, firewalled network (chosen)** | Images to build and VMs to recycle | Medium | Low: build code reaches neither customer data nor the platform's controls, and nothing persists between jobs |
| B — Runners as pods in the cluster | Shares the cluster's capacity | Small | Code from a pull request runs next to customer data |

### 6. Customer data: availability, backups, encryption and the GDPR

> Amendment 1 (2026-10-31): no backup power is assumed, and layer 3 is replaced (A2, points 8 and 9, and A6).

**Availability.** One machine is one failure domain: power, the internet
line, a disk or the motherboard.

- **Before the managed cloud opens,** trial included, all of these must be
  in place:
  - encrypted disks with off-machine keys (issues 1 and 12);
  - backups (issue 4) and the server's continuous copy with a first timed
    restore (issue 6);
  - monitoring with a probe outside the machine and a public status page
    (issue 5);
  - erasure and self-service export working in the server: deleting an
    organization or a space, exporting a whole organization, and the
    erasure log below (issue 16); a departure with its export (ADR-0022
    issue 13);
  - the GDPR documents (issue 9).

  The trial needs a card and turns into a paid plan (ADR-0014 Amendment 1),
  so it is real customer data from day one.
- **Until a second failure domain exists,** the contract states best-effort
  availability, and a UPS covers short power cuts (§9).
- **Before the public launch,** a second failure domain must exist (§7),
  and a full rebuild test must have passed (issue 14).
- **Targets:** a restore point no older than a few minutes (RPO). Until a
  second failure domain exists, losing the machine means days of outage,
  best effort, while new hardware is bought or rented. Afterwards the
  service is back within hours on the other domain (RTO). The tests below
  measure the real figures.

**Backups**, in four layers:

1. **A continuous copy of every database.** A replication process in the
   server's pod, such as Litestream, ships the changes of every database to
   an EU object store, within seconds or minutes: each space database and
   the accounts database.
   - Issue 6 checks that the tool chosen does both: it encrypts the copy
     before it leaves the machine, and it picks up a new space database
     without being told.
   - If no version does both, the fallback keeps encryption: the object
     store encrypts with a key that Sobr holds, or the replication restarts
     when a space is created.
   - The object store keeps copies in three EU zones, as ADR-0014's feature
     table promises.
   - The copy's history, which allows a restore to a past point, is kept
     30 days.
2. **etcd snapshots** every day, encrypted and sent to the same object
   store, so the cluster's own state survives the machine. They are kept
   30 days.
3. **VM rollback.** Proxmox Backup Server snapshots every VM nightly, onto a
   disk other than the VMs', and keeps them 30 days at most. It rolls back a
   whole VM after a bad upgrade. It is not a backup against losing the
   machine, and it cannot restore one space; layer 1 does that. Once a
   second site exists, it syncs there.
4. **Tests.**
   - A scheduled job restores one space from layer 1 to a spare environment
     every month. It records the time and the data lost, and alerts if it
     fails or does not run (issue 13).
   - Before the public launch, the whole platform is rebuilt from code and
     backups onto the second failure domain's hardware, and timed (issue
     14).

**Erasure.** The Sobr server writes an erasure log of every deletion it must
keep deleted:

- an organization or a space;
- a departed member's detail (ADR-0022 §10);
- a withdrawn session (ADR-0022 §3).

The log lives outside every layer that a restore brings back: write-once
objects in the object store, never on the server's volume or in a database.
It is replayed after any restore and any VM rollback, so neither brings
erased data back. Live data goes at once. Every copy expires within 30
days, the retention of all three backup layers. Each entry of the log is
kept at least as long as the longest backup retention, so no restore can
outlive the record of an erasure.

**Encryption and keys.**

- **Nodes.** The disks are encrypted with Talos disk encryption. The nodes
  fetch their keys from a key server on a small machine that Thibault
  controls, at a place other than the Proxmox host. That machine later
  holds the third quorum vote (§1). It is not the public front of §4.
  - **What a request proves.** A Talos request names the node by its
    identifier, which a thief can read from the stolen machine's
    configuration. So the key server does not rely on it alone.
  - **After a long outage,** such as a power cut, a move or a theft, the key
    server waits for Thibault's confirmation before it serves keys again.
    Rolling upgrades, where nodes reboot one by one while the machine stays
    reachable, stay automatic.
  - **Revocation.** A stolen node's keys can be revoked.
  - **Reboots.** A node cannot reboot while the key server is out of reach.
- **The small machine itself.** Its disk is encrypted with a passphrase
  typed at its rare reboots, and a written procedure changes every key if
  it is lost or stolen.
- **Offline copies.** The key server's own key, and the key of the backup
  copies, have offline copies kept by Thibault. Losing the small machine,
  or the main one, then locks nothing for good.
- **Custody.** Key custody is human-only work (issue 11).

**The GDPR.**

- **For an organization's data,** Sobr acts as a processor. ADR-0014
  condition 4 asks for EU hosting, a named DPO, standard contract clauses
  and a processing contract with each organization (art. 28), a public
  privacy policy, a documented DPIA, and working erasure and export. If
  there is a breach, Sobr tells the organizations without undue delay; they
  then have 72 hours to notify their authority (art. 33).
- **For accounts, payment details and personal spaces,** Sobr is the
  controller. A breach there is notified to the CNIL within 72 hours, and to
  the people concerned when the risk is high.
- **In all cases:**
  - a record of processing (art. 30);
  - security measures (art. 32);
  - a list of sub-processors: the object store, the DNS provider, the mail
    relay, the virtual server, the uptime probe and status page, and the
    payment provider. The others are chosen in the EU. The payment provider
    (SOB-131) may not be, so it is listed with its transfer safeguards.

These are legal documents, so they are human-only work (issue 9).

**Access.** Nobody reads a customer's database.

- Agents change the platform only through approved pull requests (SOB-73),
  with credentials that cannot read the server's secrets or volumes.
- Emergency access by Thibault is logged.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Open, trial included, once the protections, erasure, export and documents exist; a second failure domain before the public launch (chosen)** | Opening waits for issues 1, 4, 5, 6, 9, 12 and 16, and for ADR-0022 issue 13 | Medium | Low: customers are covered from day one, and the second machine has a date |
| B — Open now on one machine, best effort in the contract | None | Small | Customer data before backups or legal documents exist |
| C — Wait for the second failure domain before opening anything | The cloud opens later | Medium | Low, but delays the trial that feeds the offer |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — The four layers, with an erasure log (chosen)** | A small object-store bill, since customer data is small, plus one replication process | Medium | Low: minutes of data at most, restores proven, erasure kept |
| B — Nightly VM backups only | None | Small | Up to a day lost, nothing off the machine, and no restore of one space |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Key server on a small machine Thibault controls elsewhere, with confirmation after a long outage and offline key copies (chosen)** | A small machine to buy and place; Thibault confirms after a power cut | Medium | Low: a stolen main machine stays locked unless Thibault confirms, and no provider holds the keys |
| B — Key server on a rented EU virtual server | A few euros a month | Small | Its provider could read the disk keys, and etcd once it is the third node |
| C — Keys derived on the host (node identity or a virtual TPM) | None | Small | They are stored on the same host, so a stolen machine unlocks itself |
| D — A passphrase typed at each boot | Thibault must be there after every power cut | Small | Long outages after unattended reboots |

### 7. What a second failure domain needs

> Amendment 1 (2026-10-31): a KubeSpan link to the home site may need a port that cannot be forwarded there (A4).

**Setup:** a Proxmox install, VMs created from the infrastructure code, and
Talos configurations applied.

- **At the same site,** the server's volume moves to Longhorn across the
  two machines (§3).
- **At another site,** KubeSpan links the machines, a lost site is restored
  from the continuous copy, and Proxmox Backup Server syncs between the
  sites.
- **Either way,** a failover test proves the setup (issue 10).

**Placement:** for failover, the second machine sits at another site, and
the small machine of §6 holds the third quorum vote at a third place. A
second machine at the same site protects against hardware faults, but not
against a cut of the power or the line.

| Option | Cost (orders of magnitude; quotes needed) | Effort | Risk |
|--|--|--|--|
| **A — Buy a second machine and place it at another site (chosen)** | Roughly €1,000 to €3,000 one-off, plus electricity: about 100 W at about €0.25 per kWh is roughly €220 a year (approximate) | Medium: a place with a good line to find | Low once placed: Sobr controls the hardware |
| B — Rent a dedicated server in an EU data centre | Hetzner's AX42: €99.00 a month plus a €49.00 one-time setup fee, both excluding VAT, on its official page, checked October 31, 2026 ([Hetzner](https://www.hetzner.com/dedicated-rootserver/ax42/)) | Small | The provider can read what the server holds; it must be listed as a sub-processor |
| C — A second machine at the same site | As A, without finding a place | Small | Covers hardware faults, not a power or line cut |

**Other costs:**

- **The small machine of §6:** a mini-PC, roughly a few hundred euros
  (approximate).
- **The EU front virtual server:** a few euros a month (approximate).
- **The object store:** €8.70 to €20 per TB a month in a comparison of EU
  offers ([Sliplane](https://sliplane.io/blog/ovhcloud-object-storage-alternatives)).
  Those offers are single-site; a three-zone class costs more, so a quote is
  needed. Customer data is far below one TB.

### 8. Sizing on one machine

> Amendment 1 (2026-10-31): this sizing does not fit the starting cap; see A3.

These are starting figures, to adjust once the facts of §9 are known:

| Part | vCPU | Memory | Disk | Role |
|--|--|--|--|--|
| Proxmox host itself | — | 8 GB | — | Hypervisor and ZFS cache |
| Control plane | 2 | 4 GB | 50 GB | Kubernetes API and etcd |
| Worker 1 | 4 | 8 to 16 GB | 200 GB | Sobr server, site, functions, monitoring |
| Worker 2 | 4 | 8 to 16 GB | 200 GB | Same, and where the server moves during upgrades |
| Linux CI runner | 8 | 16 GB | 200 GB | Rust and Tauri builds |
| Windows CI runner | 8 | 16 GB | 200 GB | Windows builds (needs a Windows license) |
| Proxmox Backup Server | 2 | 4 GB | Its own disk | VM rollback |

Monitoring (about 2 vCPU and 4 GB, logs kept 7 days) runs inside the
workers' share. That adds up to about 28 vCPU and 64 to 80 GB of memory,
plus about 1 TB of mirrored disk for the VMs and a separate disk for
backups. CI runners can take turns rather than run all the time. On a
smaller machine, the CI runners are the part to cut first.

### 9. Facts for Thibault to confirm

> Amendment 1 (2026-10-31): A1 answers most of these; upload speed, a spare disk, backup power and the third place stay open.

- **Hardware:** CPU (cores), memory, disks (type, size, mirror or ZFS), and
  a spare disk for backups.
- **Place:** at home or in a data centre.
- **Line:** upload speed, and whether the public address is fixed.
- **Power:** a UPS, and how long it lasts.
- **Today's use:** whether the Ubuntu machine with nginx that serves the
  site is the Proxmox host itself or a VM on it, and which other sites it
  serves.
- **A third place** for the small machine of §6, other than the Proxmox
  host's.

## Consequences

> Amendment 1 (2026-10-31): the other site is a container that a replacement will change, and the host's risks are in A6.

**Positive:**

- The whole platform is code: the nodes (Talos), the network, the services
  and the backups, ready for SOB-73's reviewed and approved deployments.
- Functions cost nothing when idle.
- Customer data survives the loss of the machine, with at most minutes
  lost. Restores and the full rebuild are tested, and erased data stays
  erased.
- A stolen machine stays locked unless Thibault confirms, and no provider
  holds the keys.
- Sobr needs no fixed home address, and the other site is untouched.
- CI code cannot reach customer data or the platform's controls.
- A second machine joins through the same code, even at another site.

**Negative:**

- Talos, Cilium and Knative are three tools to learn.
- On one machine, an upgrade or a lost node stops the managed cloud for
  minutes, or for the time of a restore. Losing the machine means days,
  until a second failure domain exists.
- After a power cut, Thibault must confirm before the nodes get their keys.
- The managed cloud waits for the gate of §6, and the public launch also
  waits for a second failure domain.
- Several things must be bought first, each a human-only task:
  - an object store;
  - a DNS provider with an API;
  - the EU front virtual server;
  - the small key machine;
  - a Windows license;
  - possibly a UPS.
- KubeSpan relies on Sidero Labs' public discovery service, which sees only
  encrypted data and the nodes' addresses.
- Several issues wait for SOB-73 and the infrastructure repo (SOB-74).

**Neutral:**

- Self-hosting customers are not affected. They keep Docker Compose or Helm
  (Cloud and self-hosting project).
- macOS builds stay on GitHub.

## Issues to create

> Amendment 1 (2026-10-31): issue 4 builds Sobr's backup layers off the host (A2, point 9), not on the host's backup server; issue 8 (CI runner VMs) waits for a cap that allows it; issues 1 and 8 follow the host contract, and a host-level change needs Thibault's go; issue 11 adds a second disk to the purchases to consider.

The orchestrator creates these issues once this ADR is accepted.

- **Deep cleanups:** only issue 16 changes application code, the Sobr
  server, so it waits for SOB-25. The Helm chart and the plain-HTTP mode
  belong to the Cloud and self-hosting project, under the same rule.
- **Roles:** `role:infra`, `role:sre` and `role:security` have no agent file
  in `.claude/agents/` yet (SOB-78 split the work by stage), so the
  orchestrator routes them until they do.

| # | Issue | Estimate | Waits for | Labels |
|--|--|--|--|--|
| 1 | Create the Talos cluster in Proxmox VMs from code, with encrypted disks whose keys come from the key server | 5 | SOB-73 accepted; the infra repo (SOB-74); #12 | agent:claude-code, role:infra, model:opus |
| 2 | Install Cilium with its Gateway API, network policies and addresses, cert-manager, and the EU front over WireGuard | 5 | #1; the DNS and virtual-server accounts (#11) | agent:claude-code, role:infra, model:sonnet |
| 3 | Install Knative Serving with Kourier, and run one test function that scales to zero | 3 | #2 | agent:claude-code, role:infra, model:sonnet |
| 4 | Set up the object store with write-once storage for the erasure log (each entry kept at least as long as the longest backup), daily etcd snapshots kept 30 days, and Proxmox Backup Server rollback kept 30 days | 3 | #1; the object store (#11) | agent:claude-code, role:sre, model:sonnet |
| 5 | Set up monitoring, alerts, a probe outside the machine and a public status page | 5 | #2 | agent:claude-code, role:sre, model:sonnet |
| 6 | Run the Sobr server from its Helm chart, with the continuous encrypted copy of every database and its history kept 30 days, restore on an empty volume, the erasure log replayed after a restore or a rollback, and a first timed restore | 5 | #2, #4, #16; the Helm chart, the image and the plain-HTTP mode (Cloud and self-hosting project) | agent:claude-code, role:infra, model:opus |
| 7 | Move the site and the web demo to a static container behind the gateway | 2 | #2, #17; SOB-73 (deploy flow) | agent:claude-code, role:infra, model:sonnet |
| 8 | Put the CI runners in throwaway VMs on their own firewalled network | 3 | #1 | agent:claude-code, role:infra, model:opus |
| 9 | Write the managed cloud's GDPR documents: DPO, DPIA, privacy policy, processing contract, record of processing, sub-processors, breach procedures as processor and as controller | 5 | — | human-only |
| 10 | Join the second failure domain and make the small machine the third control-plane node, then run a failover test | 5 | #6, #12; the hardware (#11) | agent:claude-code, role:infra, model:opus |
| 11 | Open the accounts and buy what the platform needs: domain and DNS provider with an API, object store, EU front virtual server, small key machine, the second failure domain after quotes, Windows license, UPS; keep the offline key copies | 3 | — | human-only |
| 12 | Set up the key server on the small machine: confirmation after a long outage, revoking a node, encrypting its own disk, offline copy of its key, re-keying procedure | 3 | the small machine (#11) | agent:claude-code, role:security, model:opus |
| 13 | Schedule a monthly restore of one space as a job that records its time and data loss and alerts when it fails or does not run | 2 | #6, #5 | agent:claude-code, role:sre, model:sonnet |
| 14 | Rebuild the whole platform from code and backups onto the second failure domain, and time it, before the public launch | 3 | #10 | agent:claude-code, role:sre, model:opus |
| 15 | Fence a lost node: power its VM off through a limited Proxmox API token, then free the volume claim (a runbook until then) | 3 | #6 | agent:claude-code, role:sre, model:opus |
| 16 | In the Sobr server, delete an organization or a space, export a whole organization, and write the erasure log as write-once objects | 5 | ADR-0022 issue 6; **Deep cleanup SOB-25** | agent:claude-code, role:rust, model:opus (Cloud and self-hosting project) |
| 17 | Point Sobr's names at the front: `sobr.cc` records at the DNS provider, the site's and the app's hostnames, the 301 from the old host (ADR-0020) | 2 | #2; the domain and DNS account (#11) | agent:claude-code, role:infra, model:sonnet |

SOB-75 (self-hosted GitHub runners) follows §5 and builds on issue 8.

## Links

- [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD):
  decisions on hosting in Europe, personal data and self-hosting
- Infrastructure project; Cloud and self-hosting project
- ADR-0013, ADR-0014 (Amendment 1, condition 4, feature table), ADR-0015,
  ADR-0016, ADR-0017, ADR-0021, ADR-0022
- SOB-73 (infrastructure as code and deployment), SOB-74 (infra repo),
  SOB-75 (runners), SOB-78 (roles by stage), SOB-105 (trial with a card),
  SOB-131 (payment provider), SOB-136 and SOB-154 (the free personal space,
  ADR-0013 and ADR-0014 Amendment 2)
- [Talos discovery](https://docs.siderolabs.com/talos/v1.12/configure-your-talos-cluster/system-configuration/discovery)
- `docs/operations/team-aggregator.md`, `docs/operations/site-deploy.md`,
  `deploy/team/`

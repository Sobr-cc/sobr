# Host contract between Sobr and BrilliantStudio

> **Status**: draft of 2026-10-31 (SOB-217). It becomes final when the server
> session has read it and confirmed that it matches the host (a comment on
> SOB-217) and Thibault has approved the pull request.
> **Language**: English, in the `sobr` repo. The server session's own notes stay
> in its French repo and can link here.
> **Public repo**: this document holds no address, machine name, port or secret.
> Those are variables of the host file, set by the server session.

One Proxmox host, at Thibault's home, runs two projects: **BrilliantStudio**,
whose production is a game server with players, and **Sobr** (the lab, the CI
runners and, later, the managed cloud). Thibault decided on October 31 that they
keep two repos and share the host through this contract (SOB-213). Without it, a
Sobr change could stop the game.

Read it with ADR-0024 (the platform), ADR-0026 (how Sobr deploys), and the
decisions SOB-211 (what Sobr may run here), SOB-212 (the scope of Claude's
approvals) and SOB-213 (repos and front end). The survey of the host is in
SOB-181.

## 1. Who owns what

| Owner | Owns | Changes it through |
|--|--|--|
| **Sobr** | The VMs and containers in the Sobr pool, and Kubernetes inside them; the Sobr bridge or VLAN; the Sobr storage and backups | The `infra` repo and its deploy script (ADR-0026) |
| **BrilliantStudio** | Its game server VM, its other machines, its network, storage and backups | `brilliantstudio-infra` (French), and Thibault |
| **The host** | The hypervisor, its network and bridges, its storage, its updates and reboots, its backup server | Thibault, by hand, with a test copy first |

Everything in the Sobr row is a Proxmox pool, a range or a bridge named in the
host file below. Sobr's automation holds a Proxmox token that acts only on the
Sobr pool and has no host-level privilege, so it could not touch the other rows
even by mistake.

## 2. Names, ranges and reach

| Variable | Meaning | Value |
|--|--|--|
| `sobr_pool` | The Proxmox pool that holds every Sobr machine | Set by the server session |
| `sobr_vmid_range` | The VMIDs reserved to Sobr | Set by the server session |
| `sobr_ip_range` | The addresses reserved to Sobr | Set by the server session |
| `sobr_bridge` | The bridge or VLAN of Sobr machines | Set by the server session |
| `sobr_storage` | The storage Sobr may use, and its quota | Set by the server session |

**What Sobr machines may reach:** the internet, the object store of ADR-0024, and
the link to the front end (section 6). **What they may never reach:** the Proxmox
management network and interface, the host itself, and BrilliantStudio's machines.
CI runner VMs are on their own firewalled network, with no route to the
management network (ADR-0024 section 5, ADR-0026 section 8).

## 3. CPU and RAM

The host has limited memory shared with the game server: about 34 GB were free on
October 31 (SOB-181). So:

- Sobr has one **budget** for the sum of its machines: `sobr_ram_limit` and
  `sobr_vcpu_limit`, set by the server session. The sum of Sobr's allocated
  memory never exceeds the budget; there is no overcommit of memory.
- Every Sobr VM has a **hard memory limit** and a **CPU limit**. Sobr VMs are
  **pinned** to a set of cores, `sobr_cpu_set`, that excludes the cores of the
  single-thread game servers.
- CI runner VMs run at the lowest CPU priority, and the number of them at once is
  capped (`sobr_ci_max_vms`), so a build burst cannot slow the game.
- Raising the budget, changing the pinning or adding a pool is a **host-level
  change** (section 4).

ADR-0024 sized the platform at about 28 vCPU and 64 to 80 GB, which does not fit
34 GB free. SOB-218 revises the sizing against this budget.

## 4. Whose go a change needs

| Change | Who goes | How |
|--|--|--|
| A Sobr machine inside the Sobr pool and ranges; Kubernetes inside them | Claude approves and applies | ADR-0026: approver who is not the author, green checks, recorded plan, rollback, Thibault's stop switch |
| Raising the Sobr budget; a new bridge, VLAN or firewall rule; storage; Proxmox updates; a reboot of the host | **Thibault, explicitly** | A test copy first. These are production changes, because they touch both projects |
| Anything that touches BrilliantStudio's machines, network, storage or backups | **Thibault, explicitly**, through `brilliantstudio-infra` | Not from `infra` |
| Any restart of the game VM | **Thibault, explicitly** | Never while players are connected, and not in the evenings |

Thibault can stop all of Sobr's automation at any time (the stop switch of
ADR-0026 section 5). A plan from `infra` that touches anything outside the Sobr
pool is refused by the script and by the token's privileges.

## 5. Backups

- Sobr's backups live in the **object store** of ADR-0024, not only on the host's
  one NVMe. The Proxmox Backup Server on the same disk is a convenience, not a
  protection against losing the disk.
- They restore **on another machine**: that is tested monthly (ADR-0024 section 6)
  and is the portability check of section 7.
- Only Thibault holds the credentials that restore them (human-only, ADR-0024).
  Sessions cannot read them.
- BrilliantStudio's backups are its own. Sobr never reads, moves or deletes them.

## 6. The front end on the internet

The default is **two front ends**: one for BrilliantStudio, one for Sobr. A single
shared small front end is allowed only if all of this is written down and true:

- its routing is split in **one file per project**, each changed only by its own
  repo (`infra` for Sobr hostnames, `brilliantstudio-infra` for the others);
- a deploy of one project cannot write the other project's file;
- a change to the shared component itself (its software, its network, its key
  material) is a host-level change and needs Thibault's go (section 4).

Until the shared case is chosen and this section is updated, Sobr has its own
front end (ADR-0024 section 4). This contract does not say which of the two
Thibault prefers; it says what must hold if he chooses the shared one.

## 7. Portability (SOB-211)

Nothing of Sobr may depend on this particular host.

- All Sobr infrastructure is in code. No address, host name, VMID or storage ID is
  written by hand outside the code.
- Host-specific values are variables in one file per host. A second host is a
  second file.
- Backups restore on another machine, and are kept off the one disk.
- No Sobr feature may rely on something only this box offers. A review refuses it.

## 8. Risks accepted until the host improves

Thibault accepted these on October 31 (SOB-211), to be lifted when the matching
improvement exists:

- the host has **one disk, no RAID** and the backup server on the same disk, until
  a second disk exists;
- there is **no UPS**, until one exists;
- the line's contract limits services provided to third parties from it, until a
  business line exists (ADR-0024 records this).

## 9. How this contract changes

By a pull request to this file in the `sobr` repo. The server session reads it and
confirms that it matches the host; Thibault approves it. A change to a value in the
host file is made by the server session in the file, not in this document.

## Open

The values of section 2 and the limits of section 3 are to be set by the server
session. The sizing of ADR-0024 against the budget is SOB-218.

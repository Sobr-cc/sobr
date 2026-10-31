# Host contract between Sobr and BrilliantStudio

> **Status**: draft of 2026-10-31 (SOB-217).
> **Language**: English, in the `sobr` repo. The server session's own notes stay in
> its French repo and can link here.
> **Public repo**: this document holds no address, machine name, port, hour or
> secret. Those are variables of the host file, set by the server session.

One Proxmox host, at Thibault's home, runs two projects: **BrilliantStudio**, whose
production is a game server with players, and **Sobr** (the lab, the CI runners
and, later, the managed cloud). Thibault decided on October 31 that they keep two
repos and share the host through this contract (SOB-213). Without it, a Sobr
change could stop the game.

Read it with ADR-0024 (the platform), ADR-0026 (how Sobr deploys), and the
decisions SOB-211 (what Sobr may run here), SOB-212 (the scope of Claude's
approvals) and SOB-213 (repos and front end). The survey of the host is in
SOB-181.

## 1. Who owns what

| Owner | Owns | Changes it through |
|--|--|--|
| **Sobr** | The VMs and containers in the Sobr pool, and Kubernetes inside them; the storage and backups of Sobr; everything inside the Sobr bridge or VLAN | The `infra` repo and its deploy script (ADR-0026) |
| **BrilliantStudio** | Its game server VM, its other machines, its network, storage and backups | `brilliantstudio-infra` (French), and Thibault |
| **The host** | The hypervisor, its network, its bridges (the Sobr bridge or VLAN is created once by Thibault), its storage and quotas, its updates and reboots, its backup server | Thibault, by hand, with a test copy first |

Sobr's automation holds a Proxmox token that acts only on the Sobr pool. It has no
host-level privilege, no console access to any machine, no restore of backups, and
no access to storage outside the Sobr storage. A plan that touches anything else is
refused by the script too (ADR-0026 section 5).

## 2. Variables of the host file

Set by the server session, in one file per host. None is written here.

- `sobr_pool`: the Proxmox pool that holds every Sobr machine;
- `sobr_vmid_range` and `sobr_ip_range`: the VMIDs and addresses reserved to Sobr;
- `sobr_bridge`: the bridge or VLAN of Sobr machines;
- `sobr_storage`: the storage Sobr may use, and its quota;
- the limits of section 3, `game_blackout_hours` (section 4) and
  `sobr_backup_window` (section 5).

**What Sobr machines may reach:** the internet, the object store (ADR-0024
section 6), and the link to the front end (section 6). **What they may never
reach:** the Proxmox management network and interface, the host itself, and
BrilliantStudio's machines. CI runner VMs sit on their own firewalled network, with
no route to the management network (ADR-0024 section 5, ADR-0026 section 8).

## 3. CPU, memory, disk and network

The host's memory and cores are shared with the game server, and the figure of
what is free is in SOB-181. So Sobr has one **budget** for the sum of its machines
(ADR-0024 sized the platform in section 8; SOB-218 revises it against this budget):

- **Memory:** `sobr_ram_limit` for the sum. Every Sobr VM has a hard limit, and
  memory is fixed: no ballooning and no overcommit.
- **CPU:** `sobr_vcpu_limit` for the sum, a CPU limit on every Sobr VM, and
  **pinning** to the core set `sobr_cpu_set`, which excludes the cores of the game
  servers. I/O threads and network queues follow the same pinning.
- **Disk and network:** every Sobr VM has a disk limit (`sobr_disk_iops_limit`,
  `sobr_disk_mbps_limit`) and a network rate limit (`sobr_net_mbps_limit`), so that
  a build or a restore cannot saturate the shared disk or the shared line.
- **CI runner VMs** run at the lowest CPU priority, and their number at once is
  capped (`sobr_ci_max_vms`).

**Who checks.** Proxmox has no native quota on a pool, so the checks are Sobr's:
the daily drift check of ADR-0026 section 6 lists the machines of the Sobr pool
against the code and reports any machine outside `sobr_vmid_range`, any limit
missing or over budget; the runner controller enforces `sobr_ci_max_vms`; and a
plan that exceeds the budget is refused by the script's policy file.

## 4. Whose go a change needs

| Change | Who goes | How |
|--|--|--|
| A Sobr machine inside the Sobr pool and ranges; Kubernetes inside them | Claude approves and applies | ADR-0026 section 5: an approver who is not the author, green checks, a recorded plan, rollback, Thibault's stop switch. This supersedes the sentence of ADR-0024 that says Thibault approves from GitHub (SOB-212) |
| Raising a Sobr limit or widening a range; a new bridge, VLAN or firewall rule; storage; Proxmox updates; a reboot of the host | **Thibault, explicitly** | A test copy first. These are production changes, because they touch both projects |
| Anything that touches BrilliantStudio's machines, network, storage or backups | **Thibault, explicitly**, through `brilliantstudio-infra` | Not from `infra` |
| Any restart of the game VM, including the one a host reboot or update causes | **Thibault, explicitly** | Never while players are connected, and not during `game_blackout_hours` (in the host's local time) |

A change to a value of the host file follows the same table: the server session
makes it, and with Thibault's go when it raises a Sobr limit or widens a range.
Thibault can stop all of Sobr's automation at any time (the stop switch of
ADR-0026 section 5).

## 5. Backups

- Sobr's backups live in the **object store** (ADR-0024 section 6), not only on the
  host's one disk. The backup server on the same disk is a convenience, not a
  protection against losing the disk.
- Sobr's backup and restore jobs run in `sobr_backup_window`, outside the game's
  busy hours, and under the rate limits of section 3.
- The keys that decrypt the backups are kept offline by Thibault alone (ADR-0024
  section 6). Sessions cannot read them. The monthly restore test of ADR-0024
  section 6 uses its own credential.
- BrilliantStudio's backups are its own. Sobr never reads, moves or deletes them.

## 6. The front end on the internet

ADR-0024 section 4 chose a front end for Sobr. SOB-213 reopens that choice for one
shared small front end, and it is allowed only if all of this is written down and
true:

- its routing is split in **one file per project**, each changed only by its own
  repo (`infra` for Sobr hostnames, `brilliantstudio-infra` for the others);
- a deploy of one project cannot write the other project's file;
- a change to the shared component itself (its software, its network, its key
  material) is a host-level change and needs Thibault's go (section 4).

Until the shared case is chosen and this section is updated, Sobr has its own front
end. If it is chosen, ADR-0024 section 4 is amended (SOB-218). This contract does
not say which of the two Thibault prefers; it says what must hold if he chooses the
shared one.

## 7. Portability (SOB-211)

Nothing of Sobr may depend on this particular host.

- All Sobr infrastructure is in code. No address, host name, VMID or storage ID is
  written by hand outside the code.
- Host-specific values are variables in one file per host. A second host is a
  second file.
- Backups restore on another machine: ADR-0024 tests one space monthly in a spare
  environment, and a full rebuild before launch (section 6 and its issues).
- No Sobr feature may rely on something only this box offers. A review refuses it.

## 8. Risks accepted until the host improves

Thibault accepted these on October 31 (SOB-211), to be lifted when the matching
improvement exists:

- the host's disk has no redundancy, and the backup server shares it, until a
  second disk exists. ADR-0024 section 9 asked about this, and its assumptions of a
  separate backup disk and a mirrored disk do not hold yet (SOB-218 records the
  effect);
- there is no backup power, until it exists;
- the line's contract limits services provided to third parties from it, until a
  business line exists (SOB-218 records this in ADR-0024).

## 9. How this contract changes

By a pull request to this file in the `sobr` repo. The server session reads it and
confirms in a comment on SOB-217 that it matches the host; Thibault approves it.

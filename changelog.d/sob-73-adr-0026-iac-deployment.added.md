ADR-0026, infrastructure as code and automatic deployment (2026-10-31, SOB-73)

- **ADR-0026** (Proposed): the infrastructure lives in the private `infra` repo
  of Sobr-cc, on the Free plan, and changes only by pull request. A script
  started by cron on the Proxmox server pulls it and applies what is approved:
  OpenTofu (`bpg/proxmox`) for Proxmox, `talosctl` for the Talos nodes, Flux
  pinned to the verified commit for the cluster, SOPS with age for secrets.
  - Thibault answered the three points in his own words (SOB-207): Claude can
    also approve a deployment, which replaces the rule of October 30; the repo is
    private on Free and deployed by script and cron, with no GitHub environment,
    no paid plan and no GitHub deploy runner; CI on the self-hosted runners runs in
    batches that the orchestrator releases.
  - Because the human gate is gone, the script enforces the controls that replace
    it, and it is installed by Thibault and cannot be changed by a session: an
    approver who is never the author (a second account for reviewer sessions),
    green checks, only merged pull requests, a plan recorded before each apply, a
    guard on destruction, a daily drift check, automatic rollback after a failed
    health check, a rate limit, and a stop switch that depends on no session.
  - The ADR names the risk that remains: a session holding both tokens could
    approve its own change.
  - The approval covers only what belongs to Sobr (SOB-212): the Sobr VMs and
    containers and Kubernetes inside them. Changes to the shared host, to
    BrilliantStudio and to the game VM need Thibault's explicit go, and the
    Proxmox token of the script is limited to a Sobr pool.
  - Nothing of Sobr may depend on this particular host (SOB-211): host values are
    variables per host, backups restore elsewhere. A host contract (SOB-213,
    SOB-217) says who owns what on the host. CI runners have no route to the
    management network.
  - 10 build issues (40 points), marking those that wait for a Deep cleanup.

# ADR-0026 — Infrastructure as code and automatic deployment

- **Status**: Accepted (2026-10-31). Thibault answered the three points in his
  own words on 2026-10-31 (SOB-207). It deploys the platform of ADR-0024, which
  is Proposed until SOB-181 confirms its facts.
- **Date**: 2026-10-31
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Infrastructure project (SOB-73)
- **Related**: ADR-0019 (repos; `infra` is reserved there), ADR-0024 (SOB-72,
  the Kubernetes platform on Proxmox), ADR-0022 (the Sobr server it runs)

> **Summary.** The infrastructure lives in one private repo of Sobr-cc, `infra`,
> on the GitHub Free plan, and changes only by pull request. A script started by
> cron on the Proxmox server pulls the repo and applies what is approved:
> OpenTofu for the Proxmox layer, `talosctl` for the Talos nodes, and Flux for
> the cluster, pinned to the commit the script verified. There is no human gate:
> a session may approve and start a deployment, as Thibault decided on October 31,
> replacing the rule of October 30. Controls replace the gate, and they are
> enforced by the script, which Thibault installs on the server and which no
> session can change: an approver who is never the author, green checks, a plan
> recorded before each apply, a guard on destruction, a daily drift check,
> automatic rollback on a failed health check, and a stop switch that Thibault
> can use at any time. CI runs on throwaway VMs in batches that the orchestrator
> releases.

## Points decided

Thibault answered these three points on October 31 in the Decision issue
SOB-207, in his own words, then confirmed his reading of each.

1. **Who approves a deployment: Claude can also approve.** Sessions may
   prepare, approve and start a deployment; no human gesture is needed. This
   replaces the rule of October 30 ("deployment only after Thibault's approval
   from his GitHub account"). Merges to `main` already follow the reviewer rule.
   Chosen. Alternatives (not taken): a second account of Thibault as the only
   reviewer; sessions moving to a machine account.
2. **Where and how: a private `infra` repo on the Free plan, deployed by a
   script and cron on the Proxmox server.** No GitHub environments, no paid
   plan, no GitHub deploy runner. Chosen. Alternatives (not taken): a private
   repo on a paid plan with GitHub environments; a public repo.
3. **CI on the self-hosted runners: in batches, managed by the orchestrator.**
   Chosen. Alternatives (not taken): a click by Thibault before each run; no
   control.

The other choices are the CTO's recommendation, justified where they are made.
The controls of section 5 are the CTO's answer to the sentence "the human gate
is gone"; Thibault can tighten or loosen them by editing the policy file on the
server.

## Context and problem statement

On October 30, Thibault decided to host everything on a dedicated Proxmox
server with infrastructure "described in code", a plan shown on every pull
request, and a deployment after each merge to `main`. ADR-0024 chose the
platform: Talos Linux in Proxmox VMs configured from code with no SSH, Knative,
Cilium, cert-manager, an EU front server, and throwaway CI VMs on a firewalled
network. It leaves the tools and the flow to this ADR, and ADR-0019 reserves the
repo name `infra`.

### Facts read on the official pages (2026-10-31)

- **GitHub Free.** Branch restrictions are available in public repositories of a
  Free organization and in all repositories of a Team one, and environments
  with required reviewers work for private repositories only on Team or Pro
  ([protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
  [environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)).
  So on a private repo on the Free plan GitHub cannot stop a push to `main` or
  require a review: the rules must be enforced elsewhere. Sobr-cc is on the Free
  plan (read through the API).
- **Self-hosted runners** ([secure use](https://docs.github.com/en/actions/reference/security/secure-use)).
  They "should almost never be used for public repositories". Just-in-time
  runners "perform at most one job before being automatically removed", but
  "there is no way to guarantee that a self-hosted runner only runs one job".
  Public repositories cannot use a runner group unless its setting allows it.
- **Tools.** OpenTofu is under the MPL-2.0 licence, Terraform under the Business
  Source License 1.1 ([OpenTofu](https://github.com/opentofu/opentofu),
  [Terraform](https://github.com/hashicorp/terraform)). OpenTofu "supports
  encrypting state and plan files at rest" with a passphrase, AWS or GCP KMS,
  OpenBao or an external provider, and an `enforced` setting
  ([state encryption](https://opentofu.org/docs/language/state/encryption/)).
  The `bpg/proxmox` provider supports OpenTofu, is MPL-2.0, authenticates with
  an API token, targets Proxmox VE 9.x (8.x with limits, 7.x not supported) and
  needs SSH for "a few operations" ([repo](https://github.com/bpg/terraform-provider-proxmox)).
  `talosctl apply-config` has a dry-run mode and a `try` mode that rolls the
  configuration back after a timeout, one minute by default
  ([Talos CLI](https://docs.siderolabs.com/talos/v1.10/reference/cli)). Flux's
  kustomize-controller decrypts SOPS secrets natively, with an age key kept in a
  Kubernetes secret or a cloud KMS ([Flux guide](https://fluxcd.io/flux/guides/mozilla-sops/)).

### A fact about our own setup

Every Claude session acts on GitHub as Thibault's everyday account: reviews,
comments and assignments appear under it. A check "the approver is not the
author" is meaningless while author and approver are the same account, so it
needs two accounts (section 5).

### What is out of scope

- The platform (ADR-0024), the Sobr server and its release.
- Building the pipeline and the runners: separate issues.
- Code signing, and the app's release builds (SOB-45).

## Decision

### 1. The repo

**One repo, `infra`, in Sobr-cc** (the name ADR-0019 reserves; created by
SOB-74), **private**, on the Free plan. It holds code only: no secret in clear
and no state file.

| Path | Holds |
|--|--|
| `proxmox/` | OpenTofu for the Proxmox layer: pools, networks, VM templates, the cluster VMs, the CI runner template and pool |
| `talos/` | The machine configuration templates of ADR-0024 section 1 |
| `clusters/prod/` | The Flux manifests: platform components (ADR-0024 sections 2 to 4) and the Sobr services (section 5) |
| `gate/` | A copy of the deploy script and its policy file, for review and history. It has no effect until Thibault installs it (section 5) |
| `docs/` | Runbooks: bootstrap, break-glass, kill switch, secret rotation, state restore |

There is one environment, production. The spare environment that ADR-0024
section 6 uses for the monthly restore test is the same code with other
variables.

Because the repo is private on Free, GitHub enforces nothing on it: any account
with write access can push to `main`. The script therefore deploys only a commit
that is the merge of a pull request that passed the checks of section 5, and it
ignores any other commit on `main`.

### 2. Tools, by layer

| Layer | Tool | Why |
|--|--|--|
| Proxmox: VMs, networks, pools, templates | **OpenTofu with the `bpg/proxmox` provider** | Open licence; a plan to show on pull requests; native encryption of state |
| Talos nodes | **`talosctl apply-config`** on the files in `talos/`, as ADR-0024 section 1 says | No new tool; the `try` mode rolls a bad configuration back |
| In the cluster | **Flux**, pinned by the script to the verified commit | A small pull-based controller with no UI; it corrects drift by itself |
| Secrets in git | **SOPS with age**, decrypted by Flux | No extra component; section 7 |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — OpenTofu + `bpg/proxmox`** | HCL to learn; the provider is community-maintained | Medium | Low: open source, replaceable |
| B — Terraform | Same | Medium | A Business Source License, and a vendor that can change terms |
| C — Ansible on the Proxmox API | No state, no plan | Medium | No plan to show on a pull request; changes are steps, not a desired state |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Flux pinned by the script (chosen)** | A controller in the cluster; the script holds a kubeconfig | Small | Low: nothing in GitHub can move the cluster, and drift is corrected |
| B — Argo CD | A UI and more components | Medium | Heavier to run on one machine, against the frugal default |
| C — The script applies manifests with `kubectl` | No controller | Small | Nothing corrects drift between deployments |

The Talos Terraform provider is not chosen: ADR-0024 puts the machine
configuration in files applied by `talosctl`, and a second way to change a node
would break "code applied after review is the only way".

### 3. Where it runs

**A script started by cron on the Proxmox server** pulls the repo and applies
what is approved. It pulls; nothing connects in. Thibault chose this; two
refinements are the CTO's. The script runs in a management container or VM of
its own, not in the hypervisor's own system, so that a flaw in a provider or a
plan does not start as root on the hypervisor. And it never runs two at once
(a file lock): each run deploys the newest approved commit, and the commits
between are skipped, since each plan is made against the real state.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — A script and cron on the server (chosen)** | The script is ours to write and test | Medium | The controls are ours to get right; no GitHub setting backs them (section 5) |
| B — GitHub Actions with environments and a deploy runner | A paid plan for a private repo | Medium | Approval by a GitHub setting, but our sessions share Thibault's account, and the deploy runner holds production secrets |
| C — Atlantis or a hosted service | A service to run, or a subscription | Medium | Applies before the merge by default, or a third party holds the credentials |

### 4. The flow

1. **A pull request** changes `infra`. Every few minutes the script lists open
   pull requests. For each new head it runs a **plan in a throwaway sandbox**
   with read-only credentials, and posts the result on the pull request as a
   comment and a `plan` status: `tofu plan -lock=false` for `proxmox/`, a diff
   of the rendered Talos configurations for `talos/` (no node is contacted), and
   a diff of the rendered manifests for `clusters/`. Values marked sensitive are
   masked. Format and validation checks (`tofu fmt -check`, `tofu validate`,
   `talosctl validate`) run on GitHub's own runners as a `check` status.
2. **A reviewer session approves** with a review in state APPROVED on the head
   commit. **The merge** to `main` follows the reviewer rule.
3. **On its next run,** the script takes the newest commit on `main` and checks
   the gates of section 5. If one fails it does nothing and reports why.
4. **It plans again** on the merge commit against the real state, saves the plan
   with its hash in the log, applies that saved plan, and then applies the Talos
   configurations one node at a time in `try` mode, applying each for real once
   the node is healthy. Last it checks health, and pins Flux to the commit.
5. **It reports.** A comment on the merged pull request gives the plan hash and
   the result, and the log keeps both.

Nothing in `proxmox/`, `talos/` or `clusters/` changes except through this flow.
Direct access exists for emergencies (section 9).

### 5. The controls that replace the human gate

The script enforces these. It is installed on the server by Thibault, owned by
root, and so is its policy file (the account allow-lists and the thresholds).
`gate/` holds a copy for review, but a change there takes effect only when
Thibault installs it. That is the one human step that remains, on purpose: the
script is what replaces the gate, so no session may change it.

| Control | What it checks | What it stops |
|--|--|--|
| **The approver is never the author** | The merged pull request has a review in state APPROVED, on the commit that was merged, by an account on the approver allow-list, and that account is not the pull request's author | A session approving its own change; a change edited after its approval |
| **Checks are green** | The `plan` and `check` statuses are successful for the merged head | A merge over a failing plan or a broken configuration |
| **Only merged pull requests deploy** | The commit is the merge of a pull request | A direct push to `main`, which GitHub Free cannot block |
| **The plan is recorded** | The plan file, encrypted, and its hash are written to the log before the apply, and the apply uses that file | An apply that differs from what was recorded |
| **Destruction guard** | A plan that destroys or replaces a resource marked `protected` (VMs with data, volumes, networks, the object store) or more than a set number of resources is refused | A mistake that destroys the platform. These changes are applied by hand (section 9) |
| **Drift and health** | A daily plan on `main`, and a health check after each apply (section 6) | Silent divergence; a bad deployment staying up |
| **Rate limit** | At most a set number of applies a day | A loop of changes |
| **Stop switch** | A `halt` file on the server, read before each run and between steps. Thibault creates it by SSH or from the Proxmox console, and he can also revoke the script's token on GitHub, or stop the container. It depends on no session | Everything, at any time. The script halts itself after a failed rollback, an apply error, or a refused destruction |

**The approver accounts.** Sessions act as Thibault's everyday account (see
Context). For "the approver is not the author" to mean anything, reviewer
sessions act as a second account, a machine user, listed on the server; author
sessions keep the everyday account. The two tokens are kept in separate
credential stores.

**The risk that remains.** A session that held both tokens could approve its own
change. The controls do not make that impossible; they make it visible and
limited. Destructive changes need Thibault's hands, every deployment leaves a
comment and a log entry, and the stop switch is his. Thibault chose to remove the
human gate with this risk in view.

### 6. Drift and rollback

**Drift.**

- **In the cluster,** Flux reconciles every few minutes and corrects what
  differs from the pinned commit. It raises an alert when it cannot.
- **In the Proxmox and Talos layers,** the script runs every day, from the
  newest approved commit: `tofu plan -detailed-exitcode -lock=false`, and a
  comparison of the rendered Talos configurations with the nodes. A difference
  opens an issue in `infra`. A Claude routine cannot reach the network of the
  server, so it does not run the check; it may read the issue and file the Linear
  task, which needs judgment and no secret.

**Rollback.**

- **Automatic:** after an apply, the script checks that the Talos nodes are
  healthy, the Kubernetes API answers, and the services listed in the policy file
  are ready. If not, it re-applies the previous good commit, once. If that also
  fails, it halts and opens an issue. The `try` mode of Talos reverts a
  configuration that is not applied for real before its timeout, which the policy
  file sets longer than a node restart.
- **By hand:** revert the merge commit by a pull request; it goes through the
  same flow.
- **A VM:** restore from Proxmox Backup Server (ADR-0024 section 6, layer 3).
- **The state:** the object store keeps versions of the state file.
- **Data** is not rolled back by any of this; ADR-0024 section 6 covers it.

### 7. Secrets: where they live and who reads them

**Rule: no secret value in git in clear, and none in OpenTofu state.** Secrets
are generated outside OpenTofu (for example `talosctl gen secrets`) and
referenced by it. With no GitHub environments, the pipeline's secrets live on the
server.

| Secret | Where it lives | Who can read it |
|--|--|--|
| Proxmox API token that applies changes, the `talosconfig`, the Talos secrets bundle, the kubeconfig, the state passphrase and write credential | Root-owned files in the management container | Root on that container, which is Thibault, and the script. Sessions have no shell there; their credentials cannot read the server's secrets (ADR-0024 section 6, Access) |
| Read-only Proxmox token, read-only credential and passphrase for the state | The plan sandbox, for the length of one plan | The sandbox |
| A fine-grained GitHub token, read-only on `infra`, and a token that may only comment and set statuses | The management container | The script |
| OpenTofu state | A versioned bucket of the EU object store of ADR-0024 (its own bucket), encrypted by OpenTofu with `enforced` set | The plan and drift runs, which hold the passphrase: the state holds topology only, by the rule above |
| Secrets of the services (database, signing keys) | In git, encrypted by SOPS with age; the age private key sits in the cluster as a Kubernetes secret | Flux, in the cluster. Repo readers see ciphertext only |
| Offline copies: the age key, the state passphrase, the Talos secrets bundle | Thibault's password manager and an offline medium | Thibault |

- **Plans** run on pull request code, which can run provider code. They run in a
  throwaway sandbox with read-only credentials and no route to anything else, so
  a hostile branch could read topology and nothing more.
- The repo is private, so the ciphertext of the services' secrets is not public.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — SOPS + age (chosen)** | A key to back up and rotate | Small | Low |
| B — External Secrets with OpenBao or Vault | A secret store to run, back up and unseal | Large | More parts on one machine; a store that holds the keys to the platform |
| C — Sealed Secrets | A controller; secrets only for Kubernetes | Small | Cannot protect the pipeline's own secrets |

### 8. Runners

CI runs on throwaway VMs rebuilt from a clean image for each job (ADR-0024
section 5), registered just in time and removed after one job: Linux, and
Windows as ADR-0024 says. A small controller clones the VM when a job is queued;
choosing or writing it is a spike (CLAUDE.md section 13 asks for an ADR before a
new dependency). OpenTofu owns the template and the pool; the controller owns
the clones, which are not in the state.

- **Batches.** Thibault chose that the orchestrator releases the runs on the
  self-hosted runners in batches (SOB-207). How a batch starts is for the
  runner issue (SOB-75); the constraints are these.
- **No secret and no route to the platform.** A CI runner holds no secret beyond
  the job's own, on its own firewalled network (ADR-0024 section 5).
- **Code from our own branches only.** A pull request from a fork never runs on a
  self-hosted runner. The reusable CI workflow in `.github` (ADR-0019 section 3)
  picks the runner with `github.event.pull_request.head.repo.fork`: GitHub's
  runner for a fork, ours otherwise. A fork's pull request carries its own copy of
  the workflow file and could ask for a self-hosted label itself, so the guard
  that holds is the repo setting "Require approval for all outside collaborators"
  (the `sobr` repo asks only first-time contributors today). The `infra` repo is
  private and has no forks.
- **There is no deploy runner.** The script on the server deploys; GitHub's
  runners never hold production credentials.
- **macOS builds stay on GitHub's runners** (ADR-0024 section 5): the Apple
  licence allows macOS only on Apple hardware ([Apple](https://www.apple.com/legal/sla/)).

### 9. Bootstrap and break-glass

- **Bootstrap.** The first apply cannot run from a cluster that does not exist.
  Thibault runs it from his machine with local state, using the same commands as
  the script. The state then moves to the object store with
  `tofu init -migrate-state`, he installs the script, and later changes go through
  the flow. Cilium is installed at this stage, because nodes have no network
  without it, and Flux adopts it; the rest of ADR-0024 issue 2 goes through Flux.
  The runbook is in `docs/`.
- **Break-glass.** If the script is down or refuses a change that must happen,
  Thibault runs the same commands by hand with the offline credentials. He
  commits the change afterwards, and the daily drift check flags any difference
  until he does. Direct `talosctl` is kept for this (ADR-0024 section 1), and its
  use is logged (ADR-0024 section 6, Access).
- **Changes the script refuses** (a destruction of protected resources) are applied
  this way, after a review by Thibault.

## Examples

**A deployment.** A session opens a pull request that adds a worker VM to
`proxmox/`. The script's sandbox comments: one VM to add, nothing to change or
destroy. A reviewer session, acting as the reviewer account, approves the head
commit, and the merge follows.

1. At its next run the script finds the merge of an approved pull request, the
   author and the approver being different accounts, with green statuses.
2. It plans again, records the plan and its hash, and applies it. The Talos
   configuration of the new node goes in with `try` mode and is then applied for
   real.
3. The health check passes. The script pins Flux to the commit and comments on the
   pull request. The next day the drift check finds no difference.

**A change that is refused.** A session approves its own pull request with the
everyday account, which is also the author. At the next run the script finds that
approver and author are the same account and does nothing; it comments why.
In another case a plan would destroy the volume of a database VM: the destruction
guard refuses it, and it waits for Thibault to apply it by hand. If Thibault
creates the `halt` file, no run does anything until he removes it.

## Consequences

**Positive:**

- The infrastructure is reviewed code, and each deployment is a recorded plan.
- No production credential sits in GitHub, and no GitHub setting is needed, so
  the Free plan is enough.
- Sessions can propose, approve and deploy changes, within controls that they
  cannot change, and Thibault can stop everything.
- The cluster corrects its own drift, and a failed deployment rolls back by
  itself.
- Every tool is open source and replaceable.

**Negative:**

- The human gate is gone. A session with both the author and the reviewer
  tokens could approve its own change; the controls limit and reveal that, they
  do not forbid it (section 5).
- GitHub Free enforces no branch rule on a private repo, so `main` can receive a
  bad push. The script ignores it, but someone must revert it.
- The gate is a script we write and maintain, and a bug in it is a bug in the
  protection.
- A second GitHub account (a machine user) for reviewer sessions, and a change
  of login for them.
- The first apply and any break-glass step are manual and need runbooks.
- A controller for throwaway runner VMs has to be chosen or written.
- Plans run on pull request code in a sandbox, which has to be built and
  kept clean.

**Neutral:**

- Sobr's own release pipeline, and the app's builds (SOB-45), only change which
  runners they use.

## Facts to confirm

- **Whether a machine user can post a review in state APPROVED** that the script
  can read on a private repo on the Free plan, and whether reviews survive a squash
  merge in the API the script uses. The pages read do not say.
- **Runner groups on a Free organization.** The pages read do not say whether they
  are available. If not, runners are registered per repo.
- **Whether `talosctl apply-config --dry-run` prints a diff, and whether a
  read-only Talos role can run it.** The CLI page says it checks how the change
  "will be applied"; it does not say a diff or a role. If the role cannot, the
  daily check of the Talos layer diffs the rendered configurations against the
  repo.
- **How the script reaches Proxmox, Talos and Kubernetes** (the place and the
  line, ADR-0024 section 9, SOB-181).
- **Whether the `bpg/proxmox` resources used need SSH.** The provider says SSH is
  needed for a few operations.

## Issues to create

The orchestrator creates these issues once this ADR is accepted. "Waits for"
names the Deep cleanup of every existing area an issue touches, under the
Foundations rule. The `infra` repo is new, so it waits for none; the one that
changes existing CI waits for SOB-8 (CI green and complete).

| # | Issue | Area | Estimate | Waits for |
|--|--|--|--|--|
| 1 | Create the private `infra` repo (SOB-74, to update: on Free nothing is enforced by GitHub, the script does it) | Repo, human-only | 1 | |
| 2 | Create the machine user for reviewer sessions, put it on the approver list, and keep its token apart from the everyday account's | GitHub, human-only | 2 | #1 |
| 3 | Write `proxmox/` in OpenTofu (provider, pools, networks, templates), with encrypted state in the object store and the bootstrap runbook; ADR-0024 issue 1 then defines the cluster VMs in it | `infra` | 5 | #1; SOB-181; ADR-0024 issue 11 |
| 4 | Write the deploy script and its policy file: the gates of section 5, the recorded plan, the apply, the health check, the rollback, the `halt` file, the log; install it on the management container | `infra/gate`, server | 8 | #2; #3 |
| 5 | Write the plan sandbox and the pull request comments and statuses | `infra/gate` | 5 | #4 |
| 6 | Bootstrap Flux pinned by the script, with SOPS and age (Cilium stays in the cluster bootstrap) | `infra` | 3 | ADR-0024 issue 1; #4 |
| 7 | Write the daily drift check and the Claude routine that files the Linear task | `infra/gate` | 3 | #4 |
| 8 | Spike the controller for throwaway runner VMs (adopt or write, two options), then build the CI pool and batches with the orchestrator (SOB-75 gains this scope) | `infra`, Proxmox | 8 | #3; ADR-0024 issue 8 |
| 9 | Select the runner in the reusable CI workflows of `.github` by fork or not, and set "Require approval for all outside collaborators" on every repo | `.github`, all repos | 3 | #8; **SOB-8** (CI green and complete) |
| 10 | Write the runbooks: break-glass, the stop switch, secret rotation, state restore | `infra` docs | 2 | #4 |

## Links

- ADR-0019 (reserves `infra`), ADR-0024 (the platform), ADR-0022
- SOB-73 (this ADR), SOB-207 (the three points), SOB-72, SOB-74, SOB-75,
  SOB-17, SOB-8, SOB-45, SOB-181
- Official pages read on 2026-10-31: [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
  [environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments),
  [secure use](https://docs.github.com/en/actions/reference/security/secure-use),
  [OpenTofu state encryption](https://opentofu.org/docs/language/state/encryption/),
  [Talos CLI](https://docs.siderolabs.com/talos/v1.10/reference/cli),
  [Flux and SOPS](https://fluxcd.io/flux/guides/mozilla-sops/)

# ADR-0026 — Infrastructure as code and automatic deployment

- **Status**: Proposed (2026-10-31). Three points wait for Thibault's answers
  (SOB-207); the rest rests on the recommendations below. Merging the PR is his
  approval.
- **Date**: 2026-10-31
- **Deciders**: Thibault (decision); CTO recommendation
- **Technical context**: Infrastructure project (SOB-73)
- **Related**: ADR-0019 (repos; the `infra` repo is reserved there), ADR-0024
  (the Kubernetes platform on Proxmox, which this ADR deploys), ADR-0022
  (the Sobr server it runs)

> **Summary.** The infrastructure lives in one repo of Sobr-cc, `infra`, and
> is changed only by a pull request. OpenTofu describes the Proxmox layer,
> `talosctl` applies the Talos machine configurations of ADR-0024, and Flux
> applies the Kubernetes manifests from a branch that only an approved job can
> move. Every pull request shows the plan of its changes. After a merge to
> `main`, the deployment waits in a GitHub environment whose only required
> reviewer is an account that Thibault alone uses, with administrator bypass
> turned off. That setting, not a habit, is what stops a deployment. Secrets
> never sit in git in clear. Drift is checked every day and Flux corrects the
> cluster by itself; a rollback is a revert that goes through the same
> approval. Builds run on throwaway runner VMs that hold no secret, never for
> pull requests from forks. The deploy job runs on a separate runner, in the
> `infra` repo only.

## Points to decide

Thibault answers these three points in the Decision issue SOB-207. Each rests
on the matching section, which compares the options with their cost, effort
and risk.

1. **Who can approve a deployment** (section 4). Every Claude session acts on
   GitHub as Thibault's account, so that account cannot be the only gate.
   Recommended: a second account that he alone uses, by hand, is the only
   required reviewer.
2. **Visibility of `infra`, and the GitHub plan** (section 1). Recommended:
   private, which needs a paid plan for Sobr-cc, today on Free.
3. **Approval of ordinary CI on the self-hosted runners** (section 7).
   Recommended: none for pull requests from our own branches; always for
   deployments and for fork pull requests.

The other choices are the CTO's recommendation, justified where they are made.

## Context and problem statement

On October 30, Thibault decided to host everything on a dedicated Proxmox
server, with infrastructure "described in code", a plan shown on every pull
request, and a deployment after each merge to `main`, "approved from his
GitHub account". Builds should also run on his own server. ADR-0024 chose the
platform: Talos Linux in Proxmox VMs configured from code with no SSH,
Knative, Cilium, cert-manager, an EU front server, and throwaway CI VMs on a
firewalled network. It leaves the tools and the flow to this ADR. ADR-0019
reserves the repo name `infra` and says its content is set here.

### Facts read on the official pages (2026-10-31)

- **Environments** ([GitHub docs](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)).
  On GitHub Free, environments can be configured for public repositories only;
  private ones need Team or Pro. Required reviewers: up to six people or teams,
  and "only one of the required reviewers needs to approve". "Prevent
  self-review" stops the user who triggered a run from approving it. Deployment
  branches and tags can be limited by name. Environment secrets are available
  to a job only after the protection rules pass. "Allow administrators to bypass
  configured protection rules" can be turned off.
- **Who can approve** ([REST docs](https://docs.github.com/en/rest/actions/workflow-runs)).
  "Required reviewers with read access" may approve or reject a pending
  deployment, and a classic token needs the `repo` scope. The page does not say
  whether a GitHub App token can.
- **Self-hosted runners** ([secure use](https://docs.github.com/en/actions/reference/security/secure-use)).
  They "should almost never be used for public repositories", because anyone
  can open a pull request. Just-in-time runners "perform at most one job before
  being automatically removed", but "there is no way to guarantee that a
  self-hosted runner only runs one job". Secrets used as command-line arguments
  can be seen by another job on the same runner. Runner groups can restrict the
  repositories that use a runner, and public repositories cannot use a runner
  group unless its setting allows it.
- **Branch rules** ([protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
  [rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)).
  Branch restrictions are available in public repositories of a Free
  organization and in all repositories of a Team one. Code owner review can be
  required, and "do not allow bypassing" applies the rules to administrators.
  Rulesets are described for Team and Enterprise.
- **Tools.** OpenTofu is under the MPL-2.0 licence, and Terraform under the
  Business Source License 1.1 ([OpenTofu](https://github.com/opentofu/opentofu),
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
  kustomize-controller decrypts SOPS secrets natively, with an age key kept in
  a Kubernetes secret or a cloud KMS ([Flux guide](https://fluxcd.io/flux/guides/mozilla-sops/)).
- **The organization.** Sobr-cc is on the GitHub Free plan (read through the
  API). The `sobr` repo asks approval only from first-time contributors.

### A fact about our own setup

Every Claude session acts on GitHub as Thibault's account: reviews, comments
and assignments appear under it. With the REST rule above, a session holding
that account's token with the `repo` scope can approve a pending deployment if
that account is the required reviewer. A reviewer setting that names only
Thibault's everyday account is therefore a convention for our agents, not a
control. Section 4 deals with it.

### What is out of scope

- The platform choices (ADR-0024), and the Sobr server and its release.
- Building the pipeline and the runners: separate issues.
- Code signing, and the app's release builds (SOB-45).

## Decision

### 1. The repo

**One repo, `infra`, in Sobr-cc** (the name ADR-0019 reserves; created by
SOB-74). It holds only code, no secret in clear and no state file:

| Path | Holds |
|--|--|
| `proxmox/` | OpenTofu for the Proxmox layer: pools, networks, VM templates, the cluster VMs, the CI and deploy runner VMs |
| `talos/` | The machine configuration templates of ADR-0024 section 1 |
| `clusters/prod/` | The Flux manifests: platform components (ADR-0024 sections 2 to 4) and the Sobr services (section 5) |
| `.github/` | The plan, deploy and drift workflows |
| `docs/` | Runbooks: bootstrap, break-glass, secret rotation, state restore |

There is one environment, production. The spare environment that ADR-0024
section 6 uses for the monthly restore test is the same code with other
variables.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Private, and a paid plan for Sobr-cc (recommended)** | A price per user; environments with required reviewers work on private repos only on Team | Small | Low: the topology of a host that will hold customer data stays private, and no fork can reach the deploy runner |
| B — Public, on the Free plan | None | Small | The topology is public. Every workflow must refuse fork pull requests on self-hosted runners, and GitHub warns against them; one mistake exposes the deploy runner |

### 2. Tools, by layer

| Layer | Tool | Why |
|--|--|--|
| Proxmox: VMs, networks, pools, runner VMs | **OpenTofu with the `bpg/proxmox` provider** | Open licence; a plan to show on pull requests; native encryption of state; the provider supports OpenTofu |
| Talos nodes | **`talosctl apply-config`** on the files in `talos/`, as ADR-0024 section 1 says | No new tool; the `try` mode rolls a bad configuration back |
| In the cluster | **Flux**, reading a branch that only the approved job can move | A small pull-based controller with no UI; it needs no cluster credential outside the cluster and corrects drift by itself |
| Secrets in git | **SOPS with age**, decrypted by Flux | No extra component; section 6 |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — OpenTofu + `bpg/proxmox`** | HCL to learn; the provider is community-maintained | Medium | Low: open source, replaceable |
| B — Terraform | Same | Medium | A Business Source License, and a vendor that can change terms |
| C — Ansible on the Proxmox API | No state, no plan | Medium | No plan to show on a pull request; changes are steps, not a desired state |
| D — Pulumi | General-purpose languages | Large | A second ecosystem to run, with its own state service |

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Flux on a branch that only the approved job moves (recommended)** | One branch rule; a controller in the cluster | Small | Low: no cluster-admin credential sits in GitHub, and drift is corrected |
| B — Argo CD | A UI and more components | Medium | Heavier to run on one machine, against the frugal default |
| C — Push with `kubectl` from the deploy job | No controller | Small | A cluster-admin credential sits in GitHub, and nothing corrects drift between deployments |

The Talos Terraform provider is not chosen: ADR-0024 puts the machine
configuration in files applied by `talosctl`, and a second way to change a node
would break "code applied after review is the only way".

### 3. The flow

1. **A pull request** changes `infra`. The `plan` workflow runs on the plan
   runner (section 7) with read-only credentials from the `plan` environment:
   `tofu plan` for `proxmox/`, `talosctl apply-config --dry-run` for `talos/`,
   and a diff of the rendered manifests for `clusters/`. The result is posted
   on the pull request, and values marked sensitive are masked. A green `plan`
   is a required check.
2. **The merge** to `main` needs the review of the code owner (section 4).
3. **On the push to `main`,** the `deploy` workflow plans again on the merge
   commit and saves the plan. Its `apply` job uses `environment: production`
   and **waits**. The reviewer sees the plan, then approves. The job applies the
   saved plan, so what was shown is what runs. Then it applies the Talos
   configurations in `try` mode, one node at a time, and applies each for real
   once the node is healthy. Last it moves the `deployed` branch to the merge commit, and Flux converges the
   cluster to it.
4. **One deployment at a time** (`concurrency` on `production`, no
   cancellation).

Nothing in `proxmox/`, `talos/` or `clusters/` changes except through steps 1
to 3. Direct access exists for emergencies (section 8).

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — Workflows in GitHub Actions on our own runners (chosen)** | Workflows to write | Medium | Low: the approval is a GitHub setting, and the tools are the ones above |
| B — Atlantis | One more service to run, in the network | Medium | Applies before the merge by default, against "deploy after merge"; its approval is not a GitHub environment |
| C — A hosted service (Spacelift, env0) | A subscription | Small | A third party with credentials to Proxmox and the cluster, against hosting everything ourselves |

### 4. Who can approve, and how the setting is enforced

The approval is enforced by these GitHub settings on `infra`:

- **Environment `production`:** required reviewer = the approver account only;
  **Allow administrators to bypass: off**; deployment branches limited to
  `main`. Its secrets (section 6) reach a job only after the approval.
- **Environment `plan`:** no reviewer; read-only secrets only.
- **Branch protection on `main`:** a pull request, one approving review from
  the code owner (`CODEOWNERS`: `* @<approver>`), the `plan` check, no force
  push, **no bypass for administrators**.
- **Branch protection on `deployed`:** only the deploy job's token may push
  (a GitHub App or deploy key kept as an environment secret); nobody else.
- **Prevent self-review: off.** The merge and the approval are Thibault's two
  acts, and they may be the same person. What matters is that a session cannot
  be that person.

**The approver account.** Sessions act as Thibault's everyday account (see
Context). The reviewer must be an identity that no session can use.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — A second account that Thibault alone uses, by hand, with a passkey (recommended)** | One account to create and keep safe; one more member of Sobr-cc | Small | Low for `infra`: the sessions' account is not a reviewer, so GitHub refuses its approval; the new account's token never exists on a machine where sessions run |
| B — Sessions move to a machine account or a GitHub App, and the everyday account stays Thibault's | Every session changes its login; names on pull requests change | Large | Low, and every approval then means a human; the GitHub docs do not say whether App tokens can approve deployments |
| C — One account for everything, by convention | None | None | Any session with a `repo` token can approve a production deployment: the acceptance criterion is not met |

### 5. Drift and rollback

**Drift.**

- **In the cluster,** Flux reconciles every few minutes and corrects what
  differs from the `deployed` branch. It raises an alert when it cannot.
- **In the Proxmox and Talos layers,** a scheduled workflow runs every day on a
  deploy-side runner with read-only credentials: `tofu plan -detailed-exitcode`
  and `talosctl apply-config --dry-run`. A difference opens an issue in `infra`.
  A Claude routine cannot reach the network of the server, so it does not run
  the check; it may read the issue and file the Linear task, which needs
  judgment and no secret.

**Rollback.**

- **Code:** revert the merge commit by a pull request. It goes through the same
  plan, review and approval. Flux converges to the reverted `deployed` branch.
- **A Talos configuration:** the `try` mode of section 3 reverts it by itself
  when it is not applied for real before the timeout.
- **A VM:** restore from Proxmox Backup Server (ADR-0024 section 6, layer 3).
- **The state:** the object store keeps versions of the state file.
- **Data** is not rolled back by any of this; ADR-0024 section 6 covers it.

### 6. Secrets: where they live and who reads them

**Rule: no secret value in git in clear, and none in OpenTofu state.** Secrets
are generated outside OpenTofu (for example `talosctl gen secrets`) and
referenced by it.

| Secret | Where it lives | Who can read it |
|--|--|--|
| Proxmox API token that applies changes, the `talosconfig`, the Talos secrets bundle, the key of the App that moves `deployed`, the state passphrase | Secrets of the `production` environment | Nobody in GitHub (it never shows a secret again). The approved deploy job reads them. Thibault can replace them |
| Read-only Proxmox token, read-only `talosconfig`, state passphrase for plans | Secrets of the `plan` environment | The plan and drift jobs |
| OpenTofu state | A versioned bucket of the EU object store of ADR-0024 (its own bucket and credential), encrypted by OpenTofu with `enforced` set | Nobody without the passphrase |
| Secrets of the services (database, signing keys) | In git, encrypted by SOPS with age; the age private key sits in the cluster as a Kubernetes secret | Flux, in the cluster. Repo readers see ciphertext only |
| Offline copies: the age key, the state passphrase, the Talos secrets bundle | Thibault's password manager and an offline medium | Thibault |

- **Agents read none of these.** Their credentials cannot read the server's
  secrets (ADR-0024 section 6, Access). Environment secrets are not exposed to
  them by GitHub.
- **Plans** run on pull request code. They hold read-only credentials only, so
  a malicious pull request could read topology but could change nothing.
- **Why the repo should be private** (point 2): ciphertext of the services'
  secrets is then not public, and a later leak of the age key does not reveal
  secrets that have been in public history since the first commit.

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — SOPS + age, GitHub environment secrets for the pipeline (chosen)** | A key to back up and rotate | Small | Low |
| B — External Secrets with OpenBao or Vault | A secret store to run, back up and unseal | Large | More parts on one machine; a store that holds the keys to the platform |
| C — Sealed Secrets | A controller; secrets only for Kubernetes | Small | Cannot protect the pipeline's own secrets, so a second tool is needed anyway |

### 7. Runners

There are two kinds, both throwaway VMs rebuilt from a clean image for each job
(ADR-0024 section 5), registered just in time and removed after one job. A
small controller on the management network clones the VM when a job is queued;
choosing or writing it is a spike (CLAUDE.md section 13 asks for an ADR before
a new dependency).

| Kind | Runs | Holds | Network |
|--|--|--|--|
| **CI** (Linux, and Windows as in ADR-0024) | Build and test of the app and the repos | No secret beyond the job's own | Its own firewalled network: no Talos or Kubernetes API, no Backup Server, no Proxmox interface, no cluster network (ADR-0024) |
| **Deploy and plan** | The `plan`, `deploy` and drift workflows of `infra` | The secrets of one approved job, or read-only tokens | The management network, with access to the Proxmox, Talos and Kubernetes APIs |

- **The deploy runner is registered to the `infra` repo only,** not to the
  organization, so no other repo can send it a job. The CI runners are
  organization runners in a runner group limited to named repos.
- **Pull requests from forks never run on a self-hosted runner.** Three rules
  hold together: the repo setting "Require approval for all outside
  collaborators" (the `sobr` repo asks only first-time contributors today); the
  reusable CI workflow in `.github` (ADR-0019 section 3) picks the runner with
  `github.event.pull_request.head.repo.fork`, GitHub's own runner for a fork
  and ours otherwise; and the runner groups name the repos allowed. A fork
  pull request then runs on GitHub's runners, after Thibault approves.
- **Approval before runs.** A deployment always needs Thibault's approval
  (section 4). A fork pull request needs it on GitHub. A pull request from a
  branch of our own repos does not, because the runner is a throwaway VM with no
  secret.
- **macOS builds stay on GitHub's runners:** the Apple licence allows macOS only
  on Apple hardware ([Apple](https://www.apple.com/legal/sla/)).

| Option | Cost | Effort | Risk |
|--|--|--|--|
| **A — No approval for our own branches; approval for forks and deployments (recommended)** | None for CI | Small | Low: no secret on a CI runner; a hostile branch needs write access first |
| B — Approval before every run on a self-hosted runner | A click per pull request; agents open many | Small | None added, at a high cost in waiting |

### 8. Bootstrap and break-glass

- **Bootstrap.** The first apply cannot run from a cluster that does not
  exist. Thibault runs it from his machine with local state, using the same
  commands as the workflows. The state then moves to the object store with
  `tofu init -migrate-state`, and the first pull request after that goes
  through the normal flow. The runbook is in `docs/`.
- **Break-glass.** If the pipeline is down, Thibault can run the same
  commands by hand with the offline credentials. The change is then committed
  afterwards, and the daily drift check flags any difference until it is.
  Direct `talosctl` is kept for this (ADR-0024 section 1), and its use is
  logged.

## Examples

**A change, from pull request to cluster.** An agent opens a pull request that
adds a worker VM to `proxmox/`.

1. The `plan` workflow comments: one VM to add, nothing to change or destroy.
2. The code owner (the approver account) reviews and approves. The merge
   button works only now.
3. After the merge, the `deploy` workflow plans again and `apply` waits. Thibault
   opens the plan in GitHub, sees the same one VM, and approves. The job applies
   the saved plan.
4. A Talos configuration for the new node goes in with `try` mode and is then
   applied for real. The job moves `deployed`, and Flux has nothing more to do.
5. A day later the drift check finds no difference.

**A session tries to approve.** A session, acting as the everyday account, runs
the REST call to approve the pending `production` deployment. GitHub refuses it:
that account is not a required reviewer. With option C of section 4 it would
have gone through.

**A fork pull request.** A stranger forks `sobr` and opens a pull request that
runs a crypto miner in CI. The first run waits for Thibault's approval on
GitHub's runners. Even after he approves, the reusable workflow selects GitHub's
runner for a fork, so the job never reaches a self-hosted VM.

## Consequences

**Positive:**

- The infrastructure is reviewed code, and each deployment is a plan that a human
  approved, enforced by GitHub.
- No cluster-admin credential sits in GitHub, and the cluster corrects its own
  drift.
- Agents can propose every infrastructure change and approve none.
- Build runners hold no secret and never run fork code.
- Every tool is open source and replaceable.

**Negative:**

- A second account to keep safe (or, for option B, a change of login for every
  session).
- A paid GitHub plan, if `infra` is private.
- The deploy runner has Proxmox and cluster access while an approved job runs.
  It is rebuilt for each job, but GitHub notes that one job per runner cannot be
  guaranteed.
- Plans run on pull request code with read-only credentials, so a hostile branch
  by someone with write access can read topology.
- The first apply and any break-glass step are manual and need runbooks.
- A controller for throwaway runner VMs has to be chosen or written.

**Neutral:**

- Sobr's own release pipeline, and the app's builds (SOB-45), only change which
  runners they use.

## Facts to confirm

- **Runner groups on a Free organization.** The pages read do not say whether
  they are available. If not, runners are registered per repo, which is the
  deploy runner's design already.
- **Whether `talosctl apply-config --dry-run` prints a diff.** The CLI page says
  it checks how the change "will be applied"; it does not say a diff. If not, the
  plan job diffs the rendered configurations itself.
- **How a deploy job reaches Proxmox, Talos and Kubernetes** (the place and the
  line, ADR-0024 section 9, SOB-181).
- **Whether the plan of `bpg/proxmox` needs SSH** for the resources used. The
  provider says SSH is needed for a few operations.

## Issues to create

The orchestrator creates these issues once this ADR is accepted. "Waits for"
names the Deep cleanup of every existing area an issue touches, under the
Foundations rule. The `infra` repo is new, so it waits for none; the one that
changes existing CI waits for SOB-8.

| # | Issue | Area | Estimate | Waits for |
|--|--|--|--|--|
| 1 | Create the `infra` repo with the protections of section 4 (SOB-74, to update to match this ADR) | Repo, human-only | 1 | The answers to points 1 and 2 |
| 2 | Create the approver account with a passkey, add it to Sobr-cc, set the environments `plan` and `production`, `CODEOWNERS`, and "Require approval for all outside collaborators" on every repo | GitHub, human-only | 2 | The answer to point 1; #1 |
| 3 | Write `proxmox/` in OpenTofu (provider, pools, networks, template, cluster VMs), with encrypted state in the object store and the bootstrap runbook | `infra` | 5 | #1; SOB-181; ADR-0024 issue 11 |
| 4 | Write the `plan` workflow with the PR comment and the required check | `infra` | 3 | #3; #6 |
| 5 | Write the `deploy` workflow: plan on main, wait for approval, apply the saved plan, `talosctl` in `try` mode, move `deployed` | `infra` | 5 | #4; ADR-0024 issue 1 |
| 6 | Spike the controller for throwaway runner VMs (adopt or write, two options), then build the CI and deploy runner pools (SOB-75 gains this scope) | `infra`, Proxmox | 8 | #3; ADR-0024 issue 8 |
| 7 | Bootstrap Flux with SOPS and age, and the rule on the `deployed` branch | `infra` | 3 | ADR-0024 issues 1 and 2; #2 |
| 8 | Write the daily drift workflow, and the Claude routine that files the Linear task | `infra` | 2 | #3 |
| 9 | Select the runner in the reusable CI workflows of `.github` by fork or not, and set the runner groups | `.github`, all repos | 3 | #6; **SOB-8** |
| 10 | Write the runbooks: break-glass, secret rotation, state restore | `infra` docs | 2 | #5 |

## Links

- ADR-0019 (reserves `infra`), ADR-0024 (the platform), ADR-0022
- SOB-73 (this ADR), SOB-207 (the three points), SOB-72, SOB-74, SOB-75,
  SOB-17, SOB-8, SOB-181
- Official pages read on 2026-10-31: [environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments),
  [secure use](https://docs.github.com/en/actions/reference/security/secure-use),
  [review pending deployments](https://docs.github.com/en/rest/actions/workflow-runs),
  [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
  [rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets),
  [OpenTofu state encryption](https://opentofu.org/docs/language/state/encryption/),
  [Talos CLI](https://docs.siderolabs.com/talos/v1.10/reference/cli),
  [Flux and SOPS](https://fluxcd.io/flux/guides/mozilla-sops/)
